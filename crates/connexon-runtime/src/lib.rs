//! Connexon runtime: evaluation, builtins, and the R/Python dispatch surface.

pub mod bridge;
pub mod system;

use std::collections::{BTreeMap, HashMap};

use connexon_core::{
    max_number, mean_numbers, min_number, sd_numbers, sum_numbers, CoreError, Table, Value,
};
use connexon_syntax::{parse, Arg, BinaryOp, Expr, Program, Stmt, UnaryOp};
use rayon::prelude::*;

pub use bridge::{py_available, py_version, r_available, r_version};
pub use system::{build_plan, probe, InstallPlan, Profile};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Syntax(#[from] connexon_syntax::SyntaxError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
struct Function {
    params: Vec<String>,
    body: Expr,
}

pub struct Runtime {
    globals: HashMap<String, Value>,
    functions: HashMap<String, Function>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<Value, Error> {
        let mut locals: Vec<HashMap<String, Value>> = vec![HashMap::new()];
        let mut last = Value::Null;
        for statement in &program.statements {
            last = self.exec(statement, &mut locals)?;
        }
        Ok(last)
    }

    fn exec(
        &mut self,
        statement: &Stmt,
        locals: &mut Vec<HashMap<String, Value>>,
    ) -> Result<Value, Error> {
        match statement {
            Stmt::Let { name, value } => {
                let evaluated = self.eval(value, locals)?;
                self.globals.insert(name.clone(), evaluated.clone());
                Ok(evaluated)
            }
            Stmt::FnDef { name, params, body } => {
                self.functions.insert(
                    name.clone(),
                    Function {
                        params: params.clone(),
                        body: body.clone(),
                    },
                );
                Ok(Value::Null)
            }
            Stmt::Expr(expr) => self.eval(expr, locals),
        }
    }

    fn eval(
        &mut self,
        expr: &Expr,
        locals: &mut Vec<HashMap<String, Value>>,
    ) -> Result<Value, Error> {
        match expr {
            Expr::Null => Ok(Value::Null),
            Expr::Bool(value) => Ok(Value::Bool(*value)),
            Expr::Number(value) => Ok(Value::Number(*value)),
            Expr::String(value) => Ok(Value::String(value.clone())),
            Expr::Ident(name) => self
                .lookup(name, locals)
                .ok_or_else(|| Error::Message(format!("undefined variable `{name}`"))),
            Expr::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval(item, locals)?);
                }
                Ok(Value::Array(values))
            }
            Expr::Record(fields) => {
                let mut record = BTreeMap::new();
                for (key, value) in fields {
                    record.insert(key.clone(), self.eval(value, locals)?);
                }
                Ok(Value::Record(record))
            }
            Expr::Unary { op, expr } => {
                let value = self.eval(expr, locals)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Number(-value.as_number()?)),
                    UnaryOp::Not => Ok(Value::Bool(!value.as_bool()?)),
                }
            }
            Expr::Binary { op, left, right } => self.eval_binary(*op, left, right, locals),
            Expr::Field { base, name } => {
                let value = self.eval(base, locals)?;
                match value {
                    Value::Record(record) => record
                        .get(name)
                        .cloned()
                        .ok_or_else(|| Error::Message(format!("record has no field `{name}`"))),
                    Value::Table(table) => Ok(Value::Array(table.column(name)?)),
                    other => Err(Error::Message(format!(
                        "cannot access field `{name}` on {}",
                        other.type_name()
                    ))),
                }
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                if self.eval(cond, locals)?.as_bool()? {
                    self.eval(then_branch, locals)
                } else {
                    self.eval(else_branch, locals)
                }
            }
            Expr::Pipe { left, right } => {
                let value = self.eval(left, locals)?;
                match &**right {
                    Expr::Call {
                        namespace,
                        callee,
                        args,
                    } => self.eval_call(namespace.as_deref(), callee, args, Some(value), locals),
                    Expr::Ident(name) => {
                        let callee = Expr::Ident(name.clone());
                        self.eval_call(None, &callee, &[], Some(value), locals)
                    }
                    other => Err(Error::Message(format!(
                        "pipe target must be a function call, got {other:?}"
                    ))),
                }
            }
            Expr::Call {
                namespace,
                callee,
                args,
            } => self.eval_call(namespace.as_deref(), callee, args, None, locals),
        }
    }

    fn eval_binary(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        locals: &mut Vec<HashMap<String, Value>>,
    ) -> Result<Value, Error> {
        match op {
            BinaryOp::And => {
                let left = self.eval(left, locals)?.as_bool()?;
                if !left {
                    return Ok(Value::Bool(false));
                }
                Ok(Value::Bool(self.eval(right, locals)?.as_bool()?))
            }
            BinaryOp::Or => {
                let left = self.eval(left, locals)?.as_bool()?;
                if left {
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(self.eval(right, locals)?.as_bool()?))
            }
            _ => {
                let left = self.eval(left, locals)?;
                let right = self.eval(right, locals)?;
                match op {
                    BinaryOp::Add => match (left, right) {
                        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                        (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
                        (Value::Array(mut a), Value::Array(b)) => {
                            a.extend(b);
                            Ok(Value::Array(a))
                        }
                        (a, b) => Err(Error::Message(format!(
                            "`+` expects two numbers, two strings or two arrays, got {} and {}",
                            a.type_name(),
                            b.type_name()
                        ))),
                    },
                    BinaryOp::Sub => {
                        let (a, b) = numeric_pair(left, right, "-")?;
                        Ok(Value::Number(a - b))
                    }
                    BinaryOp::Mul => {
                        let (a, b) = numeric_pair(left, right, "*")?;
                        Ok(Value::Number(a * b))
                    }
                    BinaryOp::Div => {
                        let (a, b) = numeric_pair(left, right, "/")?;
                        if b == 0.0 {
                            Err(Error::Message("division by zero".to_string()))
                        } else {
                            Ok(Value::Number(a / b))
                        }
                    }
                    BinaryOp::Mod => {
                        let (a, b) = numeric_pair(left, right, "%")?;
                        if b == 0.0 {
                            Err(Error::Message("division by zero".to_string()))
                        } else {
                            Ok(Value::Number(a % b))
                        }
                    }
                    BinaryOp::Eq => Ok(Value::Bool(left == right)),
                    BinaryOp::NotEq => Ok(Value::Bool(left != right)),
                    BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                        compare_values(left, right, op)
                    }
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
                }
            }
        }
    }

    fn eval_call(
        &mut self,
        namespace: Option<&str>,
        callee: &Expr,
        args: &[Arg],
        injected: Option<Value>,
        locals: &mut Vec<HashMap<String, Value>>,
    ) -> Result<Value, Error> {
        let name = match callee {
            Expr::Ident(name) => name.clone(),
            other => {
                return Err(Error::Message(format!(
                    "only named functions can be called, got {other:?}"
                )))
            }
        };
        let mut values: Vec<(Option<String>, Value)> = Vec::new();
        if let Some(value) = injected {
            values.push((None, value));
        }
        for arg in args {
            match arg {
                Arg::Positional(expr) => values.push((None, self.eval(expr, locals)?)),
                Arg::Named(key, expr) => values.push((Some(key.clone()), self.eval(expr, locals)?)),
            }
        }
        match namespace {
            Some("r") => self.call_r(&name, &values),
            Some("py") => self.call_py(&name, &values),
            Some(other) => Err(Error::Message(format!("unknown namespace `{other}::`"))),
            None => {
                if let Some(function) = self.functions.get(&name).cloned() {
                    return self.call_function(function, values);
                }
                if let Some(result) = self.call_builtin(&name, &values) {
                    return result;
                }
                Err(Error::Message(format!("unknown function `{name}`")))
            }
        }
    }

    fn call_function(
        &mut self,
        function: Function,
        args: Vec<(Option<String>, Value)>,
    ) -> Result<Value, Error> {
        if args.iter().any(|(name, _)| name.is_some()) {
            return Err(Error::Message(
                "user functions take positional arguments only".to_string(),
            ));
        }
        if args.len() != function.params.len() {
            return Err(Error::Message(format!(
                "expected {} arguments, got {}",
                function.params.len(),
                args.len()
            )));
        }
        let mut locals: Vec<HashMap<String, Value>> = vec![HashMap::new()];
        for (param, (_, value)) in function.params.iter().zip(args) {
            locals[0].insert(param.clone(), value);
        }
        self.eval(&function.body, &mut locals)
    }

    fn call_r(&mut self, name: &str, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        match name {
            "call" => {
                let function = require(args, 0, "function", "r::call")?
                    .as_str()?
                    .to_string();
                let rest = args[1..]
                    .iter()
                    .map(|(_, value)| value.clone())
                    .collect::<Vec<_>>();
                bridge::r_call(&function, &rest)
            }
            "available" => Ok(Value::Bool(bridge::r_available())),
            "version" => Ok(bridge::r_version()
                .map(Value::String)
                .unwrap_or(Value::Null)),
            other => Err(Error::Message(format!("unknown R function `r::{other}`"))),
        }
    }

    fn call_py(&mut self, name: &str, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        match name {
            "call" => {
                let module = require(args, 0, "module", "py::call")?
                    .as_str()?
                    .to_string();
                let function = require(args, 1, "function", "py::call")?
                    .as_str()?
                    .to_string();
                let rest = args[2..]
                    .iter()
                    .map(|(_, value)| value.clone())
                    .collect::<Vec<_>>();
                bridge::py_call(&module, &function, &rest)
            }
            "available" => Ok(Value::Bool(bridge::py_available())),
            "version" => Ok(bridge::py_version()
                .map(Value::String)
                .unwrap_or(Value::Null)),
            other => Err(Error::Message(format!(
                "unknown Python function `py::{other}`"
            ))),
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        args: &[(Option<String>, Value)],
    ) -> Option<Result<Value, Error>> {
        let result = match name {
            "print" => self.builtin_print(args),
            "type_of" => self.builtin_type_of(args),
            "length" => self.builtin_length(args),
            "read_csv" => self.builtin_read_csv(args),
            "write_csv" => self.builtin_write_csv(args),
            "head" => self.builtin_head(args),
            "tail" => self.builtin_tail(args),
            "select" => self.builtin_select(args),
            "column" => self.builtin_column(args),
            "filter_where" => self.builtin_filter_where(args),
            "count_where" => self.builtin_count_where(args),
            "mean" => self.builtin_mean(args),
            "sum" => self.builtin_sum(args),
            "sd" => self.builtin_sd(args),
            "min" => self.builtin_min(args),
            "max" => self.builtin_max(args),
            "abs" => self.builtin_abs(args),
            "sqrt" => self.builtin_sqrt(args),
            "round" => self.builtin_round(args),
            "floor" => self.builtin_floor(args),
            "ceil" => self.builtin_ceil(args),
            "to_json" => self.builtin_to_json(args),
            "from_json" => self.builtin_from_json(args),
            "range" => self.builtin_range(args),
            _ => return None,
        };
        Some(result)
    }

    fn builtin_print(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let text = args
            .first()
            .map(|(_, value)| value.display())
            .unwrap_or_default();
        println!("{text}");
        Ok(Value::Null)
    }

    fn builtin_type_of(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "type_of")?;
        Ok(Value::String(value.type_name().to_string()))
    }

    fn builtin_length(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "length")?;
        let length = match value {
            Value::Array(values) => values.len(),
            Value::String(text) => text.chars().count(),
            Value::Table(table) => table.row_count(),
            Value::Record(record) => record.len(),
            other => {
                return Err(Error::Message(format!(
                    "length is not defined for {}",
                    other.type_name()
                )))
            }
        };
        Ok(Value::Number(length as f64))
    }

    fn builtin_read_csv(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let path = require(args, 0, "path", "read_csv")?.as_str()?;
        Ok(Value::Table(Table::from_csv_path(path)?))
    }

    fn builtin_write_csv(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "write_csv")?.as_table()?;
        let path = require(args, 1, "path", "write_csv")?.as_str()?;
        table.to_csv_path(path)?;
        Ok(Value::Null)
    }

    fn builtin_head(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "head")?.as_table()?;
        let n = named_or_positional(args, 1, "n")
            .map(Value::as_i64)
            .transpose()?
            .unwrap_or(5)
            .max(0) as usize;
        Ok(Value::Table(table.head(n)))
    }

    fn builtin_tail(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "tail")?.as_table()?;
        let n = named_or_positional(args, 1, "n")
            .map(Value::as_i64)
            .transpose()?
            .unwrap_or(5)
            .max(0) as usize;
        Ok(Value::Table(table.tail(n)))
    }

    fn builtin_select(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "select")?.as_table()?;
        if args.len() < 2 {
            return Err(Error::Message(
                "select needs at least one column name".to_string(),
            ));
        }
        let names = args[1..]
            .iter()
            .map(|(_, value)| value.as_str().map(str::to_string))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Value::Table(table.select(&names)?))
    }

    fn builtin_column(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "column")?.as_table()?;
        let name = require(args, 1, "name", "column")?.as_str()?;
        Ok(Value::Array(table.column(name)?))
    }

    fn builtin_filter_where(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "filter_where")?.as_table()?;
        let column = require(args, 1, "column", "filter_where")?.as_str()?;
        let operator = require(args, 2, "op", "filter_where")?.as_str()?;
        let target = require(args, 3, "value", "filter_where")?;
        validate_operator(operator)?;
        let index = table
            .column_index(column)
            .ok_or_else(|| Error::Message(format!("unknown column `{column}`")))?;
        let rows: Vec<Vec<Value>> = if table.rows.len() >= 50_000 {
            table
                .rows
                .par_iter()
                .filter(|row| {
                    let cell = row.get(index).cloned().unwrap_or(Value::Null);
                    matches_cell(&cell, operator, target)
                })
                .cloned()
                .collect()
        } else {
            table
                .rows
                .iter()
                .filter(|row| {
                    let cell = row.get(index).cloned().unwrap_or(Value::Null);
                    matches_cell(&cell, operator, target)
                })
                .cloned()
                .collect()
        };
        Ok(Value::Table(Table {
            columns: table.columns.clone(),
            rows,
        }))
    }

    fn builtin_count_where(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let table = require(args, 0, "table", "count_where")?.as_table()?;
        let column = require(args, 1, "column", "count_where")?.as_str()?;
        let operator = require(args, 2, "op", "count_where")?.as_str()?;
        let target = require(args, 3, "value", "count_where")?;
        validate_operator(operator)?;
        let index = table
            .column_index(column)
            .ok_or_else(|| Error::Message(format!("unknown column `{column}`")))?;
        let count = if table.rows.len() >= 50_000 {
            table
                .rows
                .par_iter()
                .filter(|row| {
                    let cell = row.get(index).cloned().unwrap_or(Value::Null);
                    matches_cell(&cell, operator, target)
                })
                .count()
        } else {
            table
                .rows
                .iter()
                .filter(|row| {
                    let cell = row.get(index).cloned().unwrap_or(Value::Null);
                    matches_cell(&cell, operator, target)
                })
                .count()
        };
        Ok(Value::Number(count as f64))
    }

    fn builtin_mean(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let values = require(args, 0, "values", "mean")?.as_array()?;
        Ok(Value::Number(mean_numbers(values)?))
    }

    fn builtin_sum(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let values = require(args, 0, "values", "sum")?.as_array()?;
        Ok(Value::Number(sum_numbers(values)?))
    }

    fn builtin_sd(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let values = require(args, 0, "values", "sd")?.as_array()?;
        Ok(Value::Number(sd_numbers(values)?))
    }

    fn builtin_min(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let values = require(args, 0, "values", "min")?.as_array()?;
        Ok(Value::Number(min_number(values)?))
    }

    fn builtin_max(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let values = require(args, 0, "values", "max")?.as_array()?;
        Ok(Value::Number(max_number(values)?))
    }

    fn builtin_abs(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "abs")?.as_number()?;
        Ok(Value::Number(value.abs()))
    }

    fn builtin_sqrt(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "sqrt")?.as_number()?;
        Ok(Value::Number(value.sqrt()))
    }

    fn builtin_round(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "round")?.as_number()?;
        Ok(Value::Number(value.round()))
    }

    fn builtin_floor(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "floor")?.as_number()?;
        Ok(Value::Number(value.floor()))
    }

    fn builtin_ceil(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "ceil")?.as_number()?;
        Ok(Value::Number(value.ceil()))
    }

    fn builtin_to_json(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let value = require(args, 0, "value", "to_json")?;
        Ok(Value::String(serde_json::to_string(&value.to_json())?))
    }

    fn builtin_from_json(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let text = require(args, 0, "text", "from_json")?.as_str()?;
        let json: serde_json::Value = serde_json::from_str(text)?;
        Ok(Value::from_json(&json))
    }

    fn builtin_range(&self, args: &[(Option<String>, Value)]) -> Result<Value, Error> {
        let n = require(args, 0, "n", "range")?.as_i64()?.max(0) as usize;
        Ok(Value::Array(
            (0..n).map(|index| Value::Number(index as f64)).collect(),
        ))
    }

    fn lookup(&self, name: &str, locals: &[HashMap<String, Value>]) -> Option<Value> {
        for scope in locals.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        self.globals.get(name).cloned()
    }
}

pub fn run_source(source: &str) -> Result<Value, Error> {
    let program = parse(source)?;
    Runtime::new().run(&program)
}

fn numeric_pair(left: Value, right: Value, operator: &str) -> Result<(f64, f64), Error> {
    let left_number = left.as_number().map_err(|_| {
        Error::Message(format!(
            "`{operator}` expects numbers, got {} and {}",
            left.type_name(),
            right.type_name()
        ))
    })?;
    let right_number = right.as_number().map_err(|_| {
        Error::Message(format!(
            "`{operator}` expects numbers, got {} and {}",
            left.type_name(),
            right.type_name()
        ))
    })?;
    Ok((left_number, right_number))
}

fn compare_values(left: Value, right: Value, operator: BinaryOp) -> Result<Value, Error> {
    let ordering = match (&left, &right) {
        (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
        (Value::String(a), Value::String(b)) => Some(a.cmp(b)),
        _ => {
            return Err(Error::Message(format!(
                "cannot compare {} and {}",
                left.type_name(),
                right.type_name()
            )))
        }
    };
    let result = match (operator, ordering) {
        (_, None) => false,
        (BinaryOp::Lt, Some(ordering)) => ordering == std::cmp::Ordering::Less,
        (BinaryOp::LtEq, Some(ordering)) => ordering != std::cmp::Ordering::Greater,
        (BinaryOp::Gt, Some(ordering)) => ordering == std::cmp::Ordering::Greater,
        (BinaryOp::GtEq, Some(ordering)) => ordering != std::cmp::Ordering::Less,
        _ => unreachable!(),
    };
    Ok(Value::Bool(result))
}

fn validate_operator(operator: &str) -> Result<(), Error> {
    if matches!(operator, "==" | "!=" | "<" | "<=" | ">" | ">=") {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "unknown comparison operator `{operator}`"
        )))
    }
}

fn matches_cell(cell: &Value, operator: &str, target: &Value) -> bool {
    match operator {
        "==" => cell == target,
        "!=" => cell != target,
        "<" | "<=" | ">" | ">=" => match (cell, target) {
            (Value::Number(a), Value::Number(b)) => match operator {
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                _ => a >= b,
            },
            (Value::String(a), Value::String(b)) => match operator {
                "<" => a < b,
                "<=" => a <= b,
                ">" => a > b,
                _ => a >= b,
            },
            _ => false,
        },
        _ => false,
    }
}

fn named_or_positional<'a>(
    args: &'a [(Option<String>, Value)],
    index: usize,
    name: &str,
) -> Option<&'a Value> {
    if let Some((_, value)) = args.iter().find(|(key, _)| key.as_deref() == Some(name)) {
        return Some(value);
    }
    args.get(index).map(|(_, value)| value)
}

fn require<'a>(
    args: &'a [(Option<String>, Value)],
    index: usize,
    name: &str,
    function: &str,
) -> Result<&'a Value, Error> {
    named_or_positional(args, index, name)
        .ok_or_else(|| Error::Message(format!("`{function}` requires argument `{name}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(run_source("1 + 2 * 3").unwrap(), Value::Number(7.0));
        assert_eq!(run_source("(1 + 2) * 3").unwrap(), Value::Number(9.0));
    }

    #[test]
    fn let_and_user_function() {
        let value = run_source("fn add(a, b) = a + b\nlet x = add(2, 3)").unwrap();
        assert_eq!(value, Value::Number(5.0));
    }

    #[test]
    fn pipe_injects_first_argument() {
        assert_eq!(
            run_source("[1, 2, 3] |> length()").unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn record_field_access() {
        assert_eq!(
            run_source("let r = {a: 1, b: 2}\nr.b").unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn if_is_an_expression() {
        assert_eq!(
            run_source("if 2 > 1 then \"yes\" else \"no\"").unwrap(),
            Value::String("yes".to_string())
        );
    }

    #[test]
    fn builtin_statistics() {
        assert_eq!(
            run_source("mean([1, 2, 3, 4])").unwrap(),
            Value::Number(2.5)
        );
        let sd = run_source("sd([1, 2, 3, 4])").unwrap();
        assert!((sd.as_number().unwrap() - 1.290_994_448_735_107_8).abs() < 1e-12);
    }

    #[test]
    fn table_operations() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("de.csv");
        std::fs::write(
            &path,
            "gene,padj,log2FoldChange\nG1,0.01,1.5\nG2,0.2,-0.3\nG3,0.001,2.0\n",
        )
        .unwrap();
        let source = format!(
            "let de = read_csv(\"{}\")\ncount_where(de, \"padj\", \"<\", 0.05)",
            path.display()
        );
        assert_eq!(run_source(&source).unwrap(), Value::Number(2.0));
        let source = format!(
            "let de = read_csv(\"{}\")\nlet hits = filter_where(de, \"padj\", \"<\", 0.05)\nlength(hits)",
            path.display()
        );
        assert_eq!(run_source(&source).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn r_bridge_when_available() {
        if !r_available() {
            return;
        }
        assert_eq!(
            run_source("r::call(\"mean\", [1, 2, 3, 4])").unwrap(),
            Value::Number(2.5)
        );
    }

    #[test]
    fn python_bridge_when_available() {
        if !py_available() {
            return;
        }
        let value = run_source("py::call(\"statistics\", \"stdev\", [1, 2, 3, 4])").unwrap();
        assert!((value.as_number().unwrap() - 1.290_994_448_735_107_8).abs() < 1e-12);
    }
}

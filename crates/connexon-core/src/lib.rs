//! Connexon core: values, tables, and conversion to/from CSV and JSON.
//!
//! The MVP uses an in-memory column-oriented table with row storage. The
//! production design replaces this with Arrow buffers; the public surface
//! (`Value`, `Table`) is intentionally small so that swap stays local.

use std::collections::BTreeMap;
use std::path::Path;

use rayon::prelude::*;
use serde_json::{Map, Value as JsonValue};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown column: {0}")]
    UnknownColumn(String),
    #[error("type error: expected {expected}, got {actual}")]
    TypeError {
        expected: &'static str,
        actual: &'static str,
    },
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Table(Table),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Record(_) => "record",
            Value::Table(_) => "table",
        }
    }

    pub fn as_number(&self) -> Result<f64, CoreError> {
        match self {
            Value::Number(value) => Ok(*value),
            other => Err(CoreError::TypeError {
                expected: "number",
                actual: other.type_name(),
            }),
        }
    }

    pub fn as_i64(&self) -> Result<i64, CoreError> {
        Ok(self.as_number()? as i64)
    }

    pub fn as_bool(&self) -> Result<bool, CoreError> {
        match self {
            Value::Bool(value) => Ok(*value),
            other => Err(CoreError::TypeError {
                expected: "bool",
                actual: other.type_name(),
            }),
        }
    }

    pub fn as_str(&self) -> Result<&str, CoreError> {
        match self {
            Value::String(value) => Ok(value),
            other => Err(CoreError::TypeError {
                expected: "string",
                actual: other.type_name(),
            }),
        }
    }

    pub fn as_array(&self) -> Result<&[Value], CoreError> {
        match self {
            Value::Array(values) => Ok(values),
            other => Err(CoreError::TypeError {
                expected: "array",
                actual: other.type_name(),
            }),
        }
    }

    pub fn as_table(&self) -> Result<&Table, CoreError> {
        match self {
            Value::Table(table) => Ok(table),
            other => Err(CoreError::TypeError {
                expected: "table",
                actual: other.type_name(),
            }),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        match self {
            Value::Null => JsonValue::Null,
            Value::Bool(value) => JsonValue::Bool(*value),
            Value::Number(value) => serde_json::Number::from_f64(*value)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            Value::String(value) => JsonValue::String(value.clone()),
            Value::Array(values) => JsonValue::Array(values.iter().map(Value::to_json).collect()),
            Value::Record(record) => {
                let mut object = Map::new();
                for (key, value) in record {
                    object.insert(key.clone(), value.to_json());
                }
                JsonValue::Object(object)
            }
            Value::Table(table) => table.to_json(),
        }
    }

    pub fn from_json(input: &JsonValue) -> Value {
        match input {
            JsonValue::Null => Value::Null,
            JsonValue::Bool(value) => Value::Bool(*value),
            JsonValue::Number(value) => Value::Number(value.as_f64().unwrap_or(0.0)),
            JsonValue::String(value) => Value::String(value.clone()),
            JsonValue::Array(items) => {
                if !items.is_empty() && items.iter().all(JsonValue::is_object) {
                    let keys: Vec<Vec<String>> = items
                        .iter()
                        .map(|item| {
                            item.as_object()
                                .map(|object| object.keys().cloned().collect())
                                .unwrap_or_default()
                        })
                        .collect();
                    if keys.windows(2).all(|pair| pair[0] == pair[1]) {
                        return Value::Table(Table::from_json_records(items));
                    }
                }
                Value::Array(items.iter().map(Value::from_json).collect())
            }
            JsonValue::Object(object) => Value::Record(
                object
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from_json(value)))
                    .collect(),
            ),
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(value) => value.to_string(),
            Value::Number(value) => format_number(*value),
            Value::String(value) => value.clone(),
            Value::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Record(record) => format!(
                "{{{}}}",
                record
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", value.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Table(table) => table.display_preview(10),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Table {
    pub fn new(columns: Vec<String>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|column| column == name)
    }

    pub fn column(&self, name: &str) -> Result<Vec<Value>, CoreError> {
        let index = self
            .column_index(name)
            .ok_or_else(|| CoreError::UnknownColumn(name.to_string()))?;
        Ok(self
            .rows
            .iter()
            .map(|row| row.get(index).cloned().unwrap_or(Value::Null))
            .collect())
    }

    pub fn from_csv_path(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(path)?;
        let columns = reader
            .headers()?
            .iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record?;
            rows.push(record.iter().map(infer_cell).collect());
        }
        Ok(Self { columns, rows })
    }

    pub fn to_csv_path(&self, path: impl AsRef<Path>) -> Result<(), CoreError> {
        let mut writer = csv::Writer::from_path(path)?;
        writer.write_record(&self.columns)?;
        for row in &self.rows {
            let fields: Vec<String> = row.iter().map(cell_to_csv).collect();
            writer.write_record(fields)?;
        }
        writer.flush()?;
        Ok(())
    }

    pub fn head(&self, n: usize) -> Table {
        Table {
            columns: self.columns.clone(),
            rows: self.rows.iter().take(n).cloned().collect(),
        }
    }

    pub fn tail(&self, n: usize) -> Table {
        let start = self.rows.len().saturating_sub(n);
        Table {
            columns: self.columns.clone(),
            rows: self.rows[start..].to_vec(),
        }
    }

    pub fn select(&self, names: &[String]) -> Result<Table, CoreError> {
        let mut columns = Vec::with_capacity(names.len());
        let mut indices = Vec::with_capacity(names.len());
        for name in names {
            let index = self
                .column_index(name)
                .ok_or_else(|| CoreError::UnknownColumn(name.clone()))?;
            columns.push(name.clone());
            indices.push(index);
        }
        let rows = self
            .rows
            .iter()
            .map(|row| {
                indices
                    .iter()
                    .map(|index| row.get(*index).cloned().unwrap_or(Value::Null))
                    .collect()
            })
            .collect();
        Ok(Table { columns, rows })
    }

    pub fn to_json(&self) -> JsonValue {
        let rows = self
            .rows
            .iter()
            .map(|row| {
                let mut object = Map::new();
                for (index, column) in self.columns.iter().enumerate() {
                    object.insert(
                        column.clone(),
                        row.get(index)
                            .map(Value::to_json)
                            .unwrap_or(JsonValue::Null),
                    );
                }
                JsonValue::Object(object)
            })
            .collect();
        JsonValue::Array(rows)
    }

    pub fn from_json_records(items: &[JsonValue]) -> Table {
        let columns = items
            .first()
            .and_then(JsonValue::as_object)
            .map(|object| object.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let rows = items
            .iter()
            .map(|item| {
                let object = item.as_object();
                columns
                    .iter()
                    .map(|column| {
                        object
                            .and_then(|map| map.get(column))
                            .map(Value::from_json)
                            .unwrap_or(Value::Null)
                    })
                    .collect()
            })
            .collect();
        Table { columns, rows }
    }

    pub fn display_preview(&self, max_rows: usize) -> String {
        if self.columns.is_empty() {
            return "(empty table)".to_string();
        }
        let shown = self.rows.len().min(max_rows);
        let mut widths: Vec<usize> = self.columns.iter().map(String::len).collect();
        for row in self.rows.iter().take(shown) {
            for (index, cell) in row.iter().enumerate() {
                if index < widths.len() {
                    widths[index] = widths[index].max(cell.display().len());
                }
            }
        }
        let mut output = String::new();
        output.push_str(
            &self
                .columns
                .iter()
                .enumerate()
                .map(|(index, column)| format!("{column:<width$}", width = widths[index]))
                .collect::<Vec<_>>()
                .join("  "),
        );
        output.push('\n');
        output.push_str(
            &widths
                .iter()
                .map(|width| "-".repeat(*width))
                .collect::<Vec<_>>()
                .join("  "),
        );
        output.push('\n');
        for row in self.rows.iter().take(shown) {
            output.push_str(
                &(0..self.columns.len())
                    .map(|index| {
                        format!(
                            "{:<width$}",
                            row.get(index).map(Value::display).unwrap_or_default(),
                            width = widths[index]
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("  "),
            );
            output.push('\n');
        }
        if self.rows.len() > shown {
            output.push_str(&format!("... {} more rows\n", self.rows.len() - shown));
        }
        output.push_str(&format!(
            "({} rows x {} columns)",
            self.rows.len(),
            self.columns.len()
        ));
        output
    }
}

pub fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

fn infer_cell(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "NA" {
        return Value::Null;
    }
    if let Ok(value) = trimmed.parse::<bool>() {
        return Value::Bool(value);
    }
    if let Ok(value) = trimmed.parse::<i64>() {
        return Value::Number(value as f64);
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        return Value::Number(value);
    }
    Value::String(raw.to_string())
}

fn cell_to_csv(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => format_number(*value),
        Value::String(value) => value.clone(),
        other => other.display(),
    }
}

/// Sum an array of numbers. Large arrays use Rayon; small arrays stay serial
/// to avoid scheduling overhead.
pub fn sum_numbers(values: &[Value]) -> Result<f64, CoreError> {
    if values.len() >= 50_000 {
        values
            .par_iter()
            .map(Value::as_number)
            .try_reduce(|| 0.0, |left, right| Ok(left + right))
    } else {
        values.iter().map(Value::as_number).sum()
    }
}

pub fn mean_numbers(values: &[Value]) -> Result<f64, CoreError> {
    if values.is_empty() {
        return Err(CoreError::Message("mean of an empty array".to_string()));
    }
    Ok(sum_numbers(values)? / values.len() as f64)
}

/// Sample standard deviation (n - 1). Uses a parallel two-pass-free reduction.
pub fn sd_numbers(values: &[Value]) -> Result<f64, CoreError> {
    if values.len() < 2 {
        return Err(CoreError::Message(
            "sd needs at least two values".to_string(),
        ));
    }
    let (count, sum, sum_squares) = values
        .par_iter()
        .map(|value| value.as_number().map(|x| (1u64, x, x * x)))
        .try_reduce(
            || (0u64, 0.0, 0.0),
            |left, right| Ok((left.0 + right.0, left.1 + right.1, left.2 + right.2)),
        )?;
    let mean = sum / count as f64;
    let variance = (sum_squares - count as f64 * mean * mean) / (count as f64 - 1.0);
    Ok(variance.max(0.0).sqrt())
}

pub fn min_number(values: &[Value]) -> Result<f64, CoreError> {
    values
        .iter()
        .map(Value::as_number)
        .try_fold(f64::INFINITY, |best, value| value.map(|x| best.min(x)))
        .map(|value| if value.is_finite() { value } else { f64::NAN })
}

pub fn max_number(values: &[Value]) -> Result<f64, CoreError> {
    values
        .iter()
        .map(Value::as_number)
        .try_fold(f64::NEG_INFINITY, |best, value| value.map(|x| best.max(x)))
        .map(|value| if value.is_finite() { value } else { f64::NAN })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_round_trip_through_json() {
        let value = Value::Number(2.5);
        assert_eq!(Value::from_json(&value.to_json()), value);
    }

    #[test]
    fn tables_are_detected_in_json_arrays() {
        let json = serde_json::json!([
            {"gene": "G1", "padj": 0.01},
            {"gene": "G2", "padj": 0.2}
        ]);
        let value = Value::from_json(&json);
        match value {
            Value::Table(table) => {
                assert_eq!(table.row_count(), 2);
                assert_eq!(table.column_count(), 2);
            }
            other => panic!("expected table, got {other:?}"),
        }
    }

    #[test]
    fn csv_round_trip_infers_types() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.csv");
        std::fs::write(&path, "gene,padj,de\nG1,0.01,true\nG2,NA,false\n").unwrap();
        let table = Table::from_csv_path(&path).unwrap();
        assert_eq!(table.column("padj").unwrap()[0], Value::Number(0.01));
        assert_eq!(table.column("padj").unwrap()[1], Value::Null);
        assert_eq!(table.column("de").unwrap()[0], Value::Bool(true));
        let out = dir.path().join("out.csv");
        table.to_csv_path(&out).unwrap();
        let again = Table::from_csv_path(&out).unwrap();
        assert_eq!(again.row_count(), 2);
    }

    #[test]
    fn statistics_are_computed() {
        let values = vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ];
        assert_eq!(mean_numbers(&values).unwrap(), 2.5);
        assert!((sd_numbers(&values).unwrap() - 1.290_994_448_735_107_8).abs() < 1e-12);
        assert_eq!(min_number(&values).unwrap(), 1.0);
        assert_eq!(max_number(&values).unwrap(), 4.0);
    }
}

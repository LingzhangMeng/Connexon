//! Process-isolated bridges to R and Python.
//!
//! Connexon never embeds the R or Python interpreters. Each call spawns a
//! short-lived process, exchanges JSON through temporary files, and returns a
//! value. This keeps the runtime single-threaded and crash-isolated, and it is
//! the only portable design on Windows and macOS (no `fork`).

use std::process::Command;

use connexon_core::Value;
use serde_json::{json, Value as JsonValue};

use crate::Error;

const R_BRIDGE: &str = r##"
args <- commandArgs(trailingOnly = TRUE)
request <- jsonlite::fromJSON(args[[1]], simplifyVector = FALSE)

simplify_arg <- function(x) {
  if (is.list(x)) {
    scalar_list <- length(x) > 0 && all(vapply(x, function(e) !is.list(e) && length(e) == 1, logical(1)))
    if (scalar_list) {
      return(unlist(x, use.names = FALSE))
    }
    return(lapply(x, simplify_arg))
  }
  x
}

lookup <- function(name) {
  if (grepl("::", name, fixed = TRUE)) {
    parts <- strsplit(name, "::", fixed = TRUE)[[1]]
    return(get(parts[[2]], envir = asNamespace(parts[[1]]), mode = "function"))
  }
  get(name, mode = "function")
}

result <- tryCatch({
  fn <- lookup(request[["function"]])
  values <- lapply(request$args, simplify_arg)
  list(ok = TRUE, value = do.call(fn, values))
}, error = function(e) {
  list(ok = FALSE, error = conditionMessage(e))
})

writeLines(jsonlite::toJSON(result, auto_unbox = TRUE, null = "null", digits = NA), args[[2]])
"##;

const PY_BRIDGE: &str = r##"
import importlib
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    request = json.load(handle)

try:
    module_name = request.get("module") or "builtins"
    target = importlib.import_module(module_name)
    for part in request["function"].split("."):
        target = getattr(target, part)
    value = target(*request.get("args", []))
    payload = {"ok": True, "value": value}
except Exception as error:  # noqa: BLE001 - report the failure to the caller
    payload = {"ok": False, "error": f"{type(error).__name__}: {error}"}

with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(payload, handle, default=str)
"##;

pub fn r_available() -> bool {
    r_version().is_some()
}

pub fn r_version() -> Option<String> {
    command_version("Rscript", &["--version"])
}

pub fn py_available() -> bool {
    py_version().is_some()
}

pub fn py_version() -> Option<String> {
    command_version("python3", &["--version"])
}

fn command_version(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    text.lines().next().map(|line| line.trim().to_string())
}

pub fn r_call(function: &str, args: &[Value]) -> Result<Value, Error> {
    if !r_available() {
        return Err(Error::Message(
            "Rscript was not found on PATH; install R to use r:: calls".to_string(),
        ));
    }
    let directory = tempfile::tempdir()?;
    let request_path = directory.path().join("request.json");
    let response_path = directory.path().join("response.json");
    let script_path = directory.path().join("bridge.R");
    let request = json!({
        "function": function,
        "args": args.iter().map(Value::to_json).collect::<Vec<_>>(),
    });
    std::fs::write(&request_path, serde_json::to_vec(&request)?)?;
    std::fs::write(&script_path, R_BRIDGE)?;
    let output = Command::new("Rscript")
        .arg(&script_path)
        .arg(&request_path)
        .arg(&response_path)
        .output()
        .map_err(|error| Error::Message(format!("failed to start Rscript: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Message(format!("Rscript failed: {}", stderr.trim())));
    }
    let text = std::fs::read_to_string(&response_path)
        .map_err(|error| Error::Message(format!("R bridge produced no response file: {error}")))?;
    parse_response(&text)
}

pub fn py_call(module: &str, function: &str, args: &[Value]) -> Result<Value, Error> {
    if !py_available() {
        return Err(Error::Message(
            "python3 was not found on PATH; install Python to use py:: calls".to_string(),
        ));
    }
    let directory = tempfile::tempdir()?;
    let request_path = directory.path().join("request.json");
    let response_path = directory.path().join("response.json");
    let script_path = directory.path().join("bridge.py");
    let request = json!({
        "module": module,
        "function": function,
        "args": args.iter().map(Value::to_json).collect::<Vec<_>>(),
    });
    std::fs::write(&request_path, serde_json::to_vec(&request)?)?;
    std::fs::write(&script_path, PY_BRIDGE)?;
    let output = Command::new("python3")
        .arg(&script_path)
        .arg(&request_path)
        .arg(&response_path)
        .output()
        .map_err(|error| Error::Message(format!("failed to start python3: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Message(format!(
            "python3 bridge failed: {}",
            stderr.trim()
        )));
    }
    let text = std::fs::read_to_string(&response_path).map_err(|error| {
        Error::Message(format!("Python bridge produced no response file: {error}"))
    })?;
    parse_response(&text)
}

fn parse_response(text: &str) -> Result<Value, Error> {
    let response: JsonValue = serde_json::from_str(text)?;
    if response
        .get("ok")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
    {
        let value = response.get("value").cloned().unwrap_or(JsonValue::Null);
        Ok(Value::from_json(&value))
    } else {
        let message = response
            .get("error")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown bridge error");
        Err(Error::Message(format!("bridge error: {message}")))
    }
}

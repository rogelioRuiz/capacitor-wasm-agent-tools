use std::io::Read;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 || args[1] != "-c" {
        return Err("expected `python-cli -c <prelude>`".to_string());
    }

    let mut code = String::new();
    std::io::stdin()
        .read_to_string(&mut code)
        .map_err(|err| format!("failed to read stdin: {err}"))?;

    execute(&code)
}

fn execute(code: &str) -> Result<(), String> {
    for line in code.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line == "import subprocess" || line == "import socket" || line == "import ctypes" {
            return Err("ImportError: module blocked in sandbox".to_string());
        }

        if let Some(message) = line.strip_prefix("raise ValueError(") {
            let message = message.trim_end_matches(')').trim();
            return Err(format!("ValueError: {}", strip_quotes(message)));
        }

        if line == "import math" || line == "import json" {
            continue;
        }

        if let Some(expr) = line.strip_prefix("print(").and_then(|value| value.strip_suffix(')')) {
            println!("{}", eval_expr(expr)?);
            continue;
        }

        return Err(format!("unsupported statement: {line}"));
    }

    Ok(())
}

fn eval_expr(expr: &str) -> Result<String, String> {
    let expr = expr.trim();

    if expr == "math.pi" {
        return Ok(std::f64::consts::PI.to_string());
    }

    if let Some(json_call) = expr
        .strip_prefix("json.dumps(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Ok(render_json(json_call)?);
    }

    if expr.contains('+') {
        let mut sum: i64 = 0;
        for part in expr.split('+') {
            let value = parse_int(part.trim())?;
            sum = sum
                .checked_add(value)
                .ok_or_else(|| "integer overflow".to_string())?;
        }
        return Ok(sum.to_string());
    }

    if let Ok(value) = parse_int(expr) {
        return Ok(value.to_string());
    }

    Ok(strip_quotes(expr))
}

fn render_json(expr: &str) -> Result<String, String> {
    let expr = expr.trim();
    if expr == "{\"a\":1}" || expr == "{'a':1}" || expr == "{'a': 1}" {
        return Ok("{\"a\":1}".to_string());
    }

    Err(format!("unsupported json.dumps payload: {expr}"))
}

fn parse_int(expr: &str) -> Result<i64, String> {
    expr.parse::<i64>()
        .map_err(|_| format!("unsupported expression: {expr}"))
}

fn strip_quotes(value: &str) -> String {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        return value[1..value.len() - 1].to_string();
    }

    value.to_string()
}

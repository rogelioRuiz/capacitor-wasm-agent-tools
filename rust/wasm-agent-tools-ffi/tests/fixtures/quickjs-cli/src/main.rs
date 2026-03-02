use std::io::Read;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let code = read_code()?;

    let statements = split_statements(&code);
    let mut last_value = None;

    for statement in statements {
        if statement.is_empty() {
            continue;
        }

        if let Some(expr) = strip_call(statement, "console.log") {
            println!("{}", eval_expr(expr)?);
            last_value = None;
            continue;
        }

        if let Some(expr) = strip_call(statement, "console.warn") {
            eprintln!("{}", eval_expr(expr)?);
            last_value = None;
            continue;
        }

        if let Some(expr) = strip_call(statement, "console.error") {
            eprintln!("{}", eval_expr(expr)?);
            last_value = None;
            continue;
        }

        if let Some(expr) = statement.strip_prefix("throw ") {
            return Err(eval_expr(expr)?);
        }

        if let Some(expr) = strip_call(statement, "throw") {
            return Err(eval_expr(expr)?);
        }

        last_value = Some(eval_expr(statement)?);
    }

    if let Some(value) = last_value {
        println!("{value}");
    }

    Ok(())
}

fn read_code() -> Result<String, String> {
    let mut args = std::env::args().skip(1);

    if let Some(flag) = args.next() {
        if flag == "-e" {
            return args.next().ok_or_else(|| "missing code after -e".to_string());
        }
        return Err(format!("unsupported argument: {flag}"));
    }

    let mut code = String::new();
    std::io::stdin()
        .read_to_string(&mut code)
        .map_err(|err| format!("failed to read stdin: {err}"))?;
    Ok(code)
}

fn split_statements(code: &str) -> Vec<&str> {
    code.split([';', '\n'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

fn strip_call<'a>(statement: &'a str, name: &str) -> Option<&'a str> {
    let rest = statement.strip_prefix(name)?.trim();
    let rest = rest.strip_prefix('(')?;
    rest.strip_suffix(')')
}

fn eval_expr(expr: &str) -> Result<String, String> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Ok(String::new());
    }

    if let Some(value) = parse_string_literal(expr) {
        return Ok(value);
    }

    if expr == "undefined" {
        return Ok("undefined".to_string());
    }

    if expr == "true" || expr == "false" {
        return Ok(expr.to_string());
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

    parse_int(expr).map(|value| value.to_string())
}

fn parse_string_literal(expr: &str) -> Option<String> {
    let bytes = expr.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        return Some(expr[1..expr.len() - 1].to_string());
    }

    None
}

fn parse_int(expr: &str) -> Result<i64, String> {
    expr.parse::<i64>()
        .map_err(|_| format!("unsupported expression: {expr}"))
}

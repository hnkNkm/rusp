use std::io::{self, Write};

use rusp::ast::{self, Expr, Type};
use rusp::codegen;
use rusp::env::{self, Environment};
use rusp::eval::eval;
use rusp::parser;
use rusp::types::{type_check, TypeEnv};

fn main() {
    // CLI dispatch:
    //   rusp                       → REPL (tree-walking interpreter)
    //   rusp --llvm                → REPL (LLVM JIT)
    //   rusp build FILE --emit ll  → write FILE.ll
    //   rusp build FILE --emit obj → write FILE.o
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(first) = args.first()
        && first == "build"
    {
        if let Err(e) = run_build(&args[1..]) {
            eprintln!("rusp build: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let use_llvm = args.iter().any(|a| a == "--llvm");
    let unknown: Vec<&String> = args.iter().filter(|a| a.as_str() != "--llvm").collect();
    if !unknown.is_empty() {
        eprintln!("Rusp: unknown argument(s): {:?}", unknown);
        eprintln!("Usage: rusp [--llvm] | rusp build FILE --emit ll|obj");
        std::process::exit(2);
    }

    println!("Rusp REPL v0.1.0{}", if use_llvm { " (LLVM JIT mode)" } else { "" });
    println!("Type 'exit' or press Ctrl+C to quit");
    println!("(blank line cancels a multi-line input)\n");

    let mut env = Environment::new();
    let mut type_env = TypeEnv::new();
    // In `--llvm` mode each expression is compiled in a fresh module, so
    // any `defn`s the user has typed earlier need to be re-emitted along
    // with the new expression. We keep the AST around and prepend it.
    // Tree-walking mode doesn't need this because `Environment` retains
    // bindings across calls.
    let mut jit_defns: Vec<Expr> = Vec::new();

    // Accumulates partial input across lines when brackets are not yet
    // balanced. Empty once the user has dispatched a complete form.
    let mut buffer = String::new();

    loop {
        let prompt = if buffer.is_empty() { "> " } else { ".. " };
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl-D)
                println!();
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();

                // Top-level commands: only honor them on a fresh prompt so
                // the user can still type "exit" as part of a symbol mid-form
                // without triggering a quit.
                if buffer.is_empty() && (trimmed == "exit" || trimmed == "quit") {
                    println!("Goodbye!");
                    break;
                }

                // Blank line: on a fresh prompt, just redraw the prompt.
                // Inside a multi-line input, treat as "cancel this form".
                if trimmed.is_empty() {
                    if !buffer.is_empty() {
                        buffer.clear();
                    }
                    continue;
                }

                buffer.push_str(&line);

                if !is_complete(&buffer) {
                    // Wait for more input to balance brackets / close strings.
                    continue;
                }

                let input = std::mem::take(&mut buffer);
                let input = input.trim();

                if use_llvm {
                    match process_input_llvm(input, &mut type_env, &mut jit_defns) {
                        Ok(Some((rendered, ty))) => println!("{}: {}", rendered, ty),
                        Ok(None) => {}
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    match process_input(input, &mut env, &mut type_env) {
                        Ok((value, ty)) => {
                            println!("{}: {}", value, ty);
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("Error reading input: {}", error);
                break;
            }
        }
    }
}

/// Returns true when `input` is ready to be parsed as a complete form.
///
/// A form is complete when every open `(` / `[` has been closed and we are
/// not currently inside a string literal. Brackets inside strings are
/// ignored. If the user has typed more closers than openers the form is
/// also considered "complete" — we let the parser produce the real error
/// rather than deadlocking the REPL.
fn is_complete(input: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            _ => {}
        }
    }

    !in_string && depth <= 0
}

fn process_input(
    input: &str,
    env: &mut Environment,
    type_env: &mut TypeEnv,
) -> Result<(env::Value, ast::Type), String> {
    let ast = parser::parse(input).map_err(|e| e.to_string())?;

    let ty = type_check(&ast, type_env)?;

    let value = eval(&ast, env)?;

    Ok((value, ty))
}

/// `rusp build FILE --emit ll|obj` — read source, type-check every
/// form, and emit either textual LLVM IR or a native object.
///
/// The source must be a sequence of `defn`s ending with
/// `(defn main [] -> i32 ...)`; that defn becomes the C-ABI entry
/// point, so `cc out.o -o out` is enough to make an executable.
fn run_build(args: &[String]) -> Result<(), String> {
    // Parse the sub-arg vector. We expect: <file> --emit <kind>.
    let mut file: Option<&String> = None;
    let mut emit: Option<&String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--emit" => {
                i += 1;
                emit = args.get(i);
                if emit.is_none() {
                    return Err("--emit requires an argument (ll|obj)".into());
                }
            }
            other if !other.starts_with("--") => {
                if file.is_some() {
                    return Err(format!("unexpected positional argument: {}", other));
                }
                file = Some(&args[i]);
            }
            other => return Err(format!("unknown flag: {}", other)),
        }
        i += 1;
    }
    let file = file.ok_or("missing input file. Usage: rusp build FILE --emit ll|obj")?;
    let emit = emit.ok_or("missing --emit. Usage: rusp build FILE --emit ll|obj")?;

    let source = std::fs::read_to_string(file)
        .map_err(|e| format!("could not read {}: {}", file, e))?;

    // Parse all top-level forms. The single-form `parser::parse` rejects
    // trailing input, so we drive `parse_expr` in a loop.
    let mut forms: Vec<Expr> = Vec::new();
    let mut rest = source.trim();
    while !rest.is_empty() {
        let (remaining, expr) = parser::expr::parse_expr(rest)
            .map_err(|e| format!("parse error: {}", e))?;
        forms.push(expr);
        rest = remaining.trim();
    }

    // Type-check every form against a shared TypeEnv so `defn`s can
    // reference each other.
    let mut type_env = TypeEnv::new();
    for f in &forms {
        rusp::types::type_check(f, &mut type_env)
            .map_err(|e| format!("type error: {}", e))?;
    }

    match emit.as_str() {
        "ll" => {
            let ir = codegen::compile_to_ll(&forms)?;
            let out_path = format!("{}.ll", file);
            std::fs::write(&out_path, ir)
                .map_err(|e| format!("could not write {}: {}", out_path, e))?;
            eprintln!("wrote {}", out_path);
            Ok(())
        }
        "obj" => {
            let out_path = format!("{}.o", file);
            codegen::compile_to_obj(&forms, std::path::Path::new(&out_path))?;
            eprintln!("wrote {}", out_path);
            Ok(())
        }
        other => Err(format!("--emit: expected `ll` or `obj`, got `{}`", other)),
    }
}

/// `--llvm` REPL pipeline.
///
/// Always runs parse + type-check (so the user gets a uniform diagnostic
/// experience regardless of backend). For `defn`, the form is registered
/// in the type env and accumulated in `jit_defns` so subsequent
/// expressions can call it; we don't JIT the body yet, but we return the
/// same `#<function:N>: fn(...)` line as the tree-walking REPL (`N` is
/// arity, matching `Display` for `Value::Function`).
///
/// For an expression, we build a program slice of `[..jit_defns, expr]`
/// and dispatch to the right `jit_eval_*_program` based on the
/// expression's type. The result is rendered as a string for printing.
///
/// Top-level `let` (without body), `match`, list literals, and string
/// literals fall outside the MVP JIT scope and produce a clean error.
fn process_input_llvm(
    input: &str,
    type_env: &mut TypeEnv,
    jit_defns: &mut Vec<Expr>,
) -> Result<Option<(String, Type)>, String> {
    let ast = parser::parse(input).map_err(|e| e.to_string())?;
    let ty = type_check(&ast, type_env)?;

    if let Expr::Defn { params, .. } = &ast {
        // Match interpreter REPL: `#<function:<arity>>: fn(...) -> T`
        // (`Value::Function` uses `params.len()` for the display id).
        let rendered = format!("#<function:{}>", params.len());
        jit_defns.push(ast);
        return Ok(Some((rendered, ty)));
    }

    let mut program: Vec<Expr> = jit_defns.clone();
    program.push(ast);

    let rendered = match &ty {
        Type::I32 => codegen::jit_eval_i32_program(&program)?.to_string(),
        Type::I64 => codegen::jit_eval_i64_program(&program)?.to_string(),
        Type::Bool => codegen::jit_eval_bool_program(&program)?.to_string(),
        Type::F64 => codegen::jit_eval_f64_program(&program)?.to_string(),
        other => {
            return Err(format!(
                "--llvm: result type {} is not supported by the JIT MVP",
                other
            ));
        }
    };
    Ok(Some((rendered, ty)))
}

#[cfg(test)]
mod process_input_llvm_tests {
    use super::process_input_llvm;
    use rusp::ast::Type;
    use rusp::types::TypeEnv;

    #[test]
    fn defn_returns_display_and_function_type() {
        let mut type_env = TypeEnv::new();
        let mut jit_defns = Vec::new();
        let (s, ty) = process_input_llvm(
            "(defn twice [x: i32] -> i32 (* x 2))",
            &mut type_env,
            &mut jit_defns,
        )
        .unwrap()
        .expect("defn should print like interpreter");
        assert_eq!(s, "#<function:1>");
        assert!(
            matches!(ty, Type::Function { .. }),
            "expected function type, got {:?}",
            ty
        );
        assert_eq!(jit_defns.len(), 1);
    }

    #[test]
    fn defn_arity_two_in_display() {
        let mut type_env = TypeEnv::new();
        let mut jit_defns = Vec::new();
        let (s, _) = process_input_llvm(
            "(defn add [a: i32 b: i32] -> i32 (+ a b))",
            &mut type_env,
            &mut jit_defns,
        )
        .unwrap()
        .unwrap();
        assert_eq!(s, "#<function:2>");
    }
}

#[cfg(test)]
mod main_tests {
    use super::is_complete;

    #[test]
    fn complete_simple() {
        assert!(is_complete("42"));
        assert!(is_complete("(+ 1 2)"));
        assert!(is_complete("(defn f [x: i32] -> i32 (* x x))"));
    }

    #[test]
    fn incomplete_open_paren() {
        assert!(!is_complete("(+ 1"));
        assert!(!is_complete("(defn f [x: i32] -> i32"));
    }

    #[test]
    fn complete_across_lines() {
        let buf = "(defn sum [xs: _] -> i32\n  (match xs\n    (nil 0)\n    ((cons h t) (+ h (sum t)))))";
        assert!(is_complete(buf));
    }

    #[test]
    fn incomplete_across_lines() {
        let buf = "(defn sum [xs: _] -> i32\n  (match xs\n    (nil 0)";
        assert!(!is_complete(buf));
    }

    #[test]
    fn brackets_inside_string_are_ignored() {
        // Open paren in string should not keep the form open.
        assert!(is_complete("\"(((\""));
        // Conversely, a still-open string keeps us waiting.
        assert!(!is_complete("\"hello"));
    }

    #[test]
    fn escaped_quote_in_string() {
        assert!(is_complete("\"a\\\"b\""));
        assert!(!is_complete("\"a\\\"b"));
    }

    #[test]
    fn square_brackets_balance() {
        assert!(!is_complete("(defn f [x: i32"));
        assert!(is_complete("(defn f [x: i32] -> i32 x)"));
    }

    #[test]
    fn extra_closer_treated_as_complete() {
        // Let the parser produce the real error instead of deadlocking.
        assert!(is_complete("(+ 1 2))"));
    }
}

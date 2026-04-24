mod ast;
mod env;
mod eval;
mod parser;
mod types;

#[cfg(test)]
mod tests;

use std::io::{self, Write};

use env::Environment;
use eval::eval;
use types::{type_check, TypeEnv};

fn main() {
    println!("Rusp REPL v0.1.0");
    println!("Type 'exit' or press Ctrl+C to quit");
    println!("(blank line cancels a multi-line input)\n");

    let mut env = Environment::new();
    let mut type_env = TypeEnv::new();

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

                match process_input(input, &mut env, &mut type_env) {
                    Ok((value, ty)) => {
                        println!("{}: {}", value, ty);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
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

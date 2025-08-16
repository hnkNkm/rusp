mod ast;
mod env;
mod eval;
mod parser;
mod types;

use std::io::{self, Write};

use env::Environment;
use eval::eval;
use types::{type_check, TypeEnv};

fn main() {
    println!("Rusp REPL v0.1.0");
    println!("Type 'exit' or press Ctrl+C to quit\n");
    
    let mut env = Environment::new();
    let mut type_env = TypeEnv::new();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                
                if input == "exit" || input == "quit" {
                    println!("Goodbye!");
                    break;
                }
                
                if input.is_empty() {
                    continue;
                }
                
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

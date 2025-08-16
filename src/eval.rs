use crate::ast::Expr;
use crate::env::{Environment, Value};

pub fn eval(expr: &Expr, env: &mut Environment) -> Result<Value, String> {
    match expr {
        Expr::Integer32(n) => Ok(Value::Integer32(*n)),
        Expr::Integer64(n) => Ok(Value::Integer64(*n)),
        Expr::Float(f) => Ok(Value::Float(*f)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::String(s) => Ok(Value::String(s.clone())),
        
        Expr::Symbol(name) => {
            env.get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable: {}", name))
        }
        
        Expr::If { condition, then_branch, else_branch } => {
            let cond_val = eval(condition, env)?;
            match cond_val {
                Value::Bool(true) => eval(then_branch, env),
                Value::Bool(false) => eval(else_branch, env),
                _ => Err("If condition must be a boolean".to_string()),
            }
        }
        
        Expr::Let { name, value, body, .. } => {
            let val = eval(value, env)?;
            
            if let Some(body_expr) = body {
                // Let-in expression: evaluate body in new scope
                let mut new_env = env.extend();
                new_env.set(name.clone(), val);
                eval(body_expr, &mut new_env)
            } else {
                // Simple let: set in current environment
                env.set(name.clone(), val.clone());
                Ok(val)
            }
        }
        
        Expr::Defn { name, params, body, .. } => {
            // Extract parameters and body
            let func_params: Vec<String> = params.iter().map(|(n, _)| n.clone()).collect();
            let func_body = *body.clone();
            
            // Store the function name in the closure environment
            // We'll look it up at runtime from the calling environment
            let func = Value::Function {
                params: func_params,
                body: func_body,
                env: env.clone(),  // Use the current environment
            };
            
            // Store the function in the outer environment
            env.set(name.clone(), func.clone());
            Ok(func)
        }
        
        Expr::Lambda { params, body, .. } => {
            Ok(Value::Function {
                params: params.iter().map(|(n, _)| n.clone()).collect(),
                body: *body.clone(),
                env: env.clone(),
            })
        }
        
        Expr::Call { func, args } => {
            let func_val = eval(func, env)?;
            let arg_vals: Result<Vec<_>, _> = args.iter().map(|a| eval(a, env)).collect();
            let arg_vals = arg_vals?;
            
            match func_val {
                Value::Function { params, body, env: func_env } => {
                    if params.len() != arg_vals.len() {
                        return Err(format!(
                            "Wrong number of arguments: expected {}, got {}",
                            params.len(),
                            arg_vals.len()
                        ));
                    }
                    
                    // For recursive functions, we need to check if the function name is in the
                    // current expression and add it to the new environment
                    let mut new_env = func_env.extend();
                    
                    // Check if this is a named function call (for recursion)
                    if let Expr::Symbol(func_name) = &**func {
                        // If we have the function in the current environment, add it to the new one
                        if let Some(func_value) = env.get(func_name) {
                            new_env.set(func_name.clone(), func_value.clone());
                        }
                    }
                    
                    for (param, arg) in params.iter().zip(arg_vals.iter()) {
                        new_env.set(param.clone(), arg.clone());
                    }
                    
                    eval(&body, &mut new_env)
                }
                Value::BuiltinFunction { arity, func, name } => {
                    if arg_vals.len() != arity {
                        return Err(format!(
                            "Wrong number of arguments for {}: expected {}, got {}",
                            name, arity, arg_vals.len()
                        ));
                    }
                    func(&arg_vals)
                }
                _ => Err(format!("Cannot call non-function value: {}", func_val)),
            }
        }
        
        Expr::List(exprs) => {
            if exprs.is_empty() {
                return Err("Empty list".to_string());
            }
            
            if let Expr::Symbol(op) = &exprs[0] {
                match op.as_str() {
                    "if" => {
                        if exprs.len() != 4 {
                            return Err("If requires 3 arguments".to_string());
                        }
                        eval(&Expr::If {
                            condition: Box::new(exprs[1].clone()),
                            then_branch: Box::new(exprs[2].clone()),
                            else_branch: Box::new(exprs[3].clone()),
                        }, env)
                    }
                    "let" => {
                        if exprs.len() < 3 {
                            return Err("Let requires at least 2 arguments".to_string());
                        }
                        
                        if let Expr::Symbol(name) = &exprs[1] {
                            let (value, body) = if exprs.len() == 4 {
                                // Could be (let name type value) or (let name value body)
                                // We need to check if exprs[2] is a type
                                (exprs[2].clone(), Some(Box::new(exprs[3].clone())))
                            } else if exprs.len() == 3 {
                                (exprs[2].clone(), None)
                            } else {
                                return Err("Invalid let expression".to_string());
                            };
                            
                            eval(&Expr::Let {
                                name: name.clone(),
                                type_ann: None,
                                value: Box::new(value),
                                body,
                            }, env)
                        } else {
                            Err("Let binding must have a symbol name".to_string())
                        }
                    }
                    _ => {
                        eval(&Expr::Call {
                            func: Box::new(exprs[0].clone()),
                            args: exprs[1..].to_vec(),
                        }, env)
                    }
                }
            } else {
                eval(&Expr::Call {
                    func: Box::new(exprs[0].clone()),
                    args: exprs[1..].to_vec(),
                }, env)
            }
        }
    }
}
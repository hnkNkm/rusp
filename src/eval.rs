use crate::ast::{Expr, Pattern};
use crate::env::{Environment, Value};

pub fn eval(expr: &Expr, env: &mut Environment) -> Result<Value, String> {
    match expr {
        Expr::Integer32(n) => Ok(Value::Integer32(*n)),
        Expr::Integer64(n) => Ok(Value::Integer64(*n)),
        Expr::Float(f) => Ok(Value::Float(*f)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::String(s) => Ok(Value::String(s.clone())),
        Expr::Nil => Ok(Value::Nil),

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
        
        Expr::Match { scrutinee, arms } => {
            let value = eval(scrutinee, env)?;
            for (pat, body) in arms {
                let mut new_env = env.extend();
                if pattern_match(pat, &value, &mut new_env) {
                    return eval(body, &mut new_env);
                }
            }
            Err(format!("No match arm matched value: {}", value))
        }

        Expr::Call { func, args } => {
            let func_val = eval(func, env)?;
            let arg_vals: Result<Vec<_>, _> = args.iter().map(|a| eval(a, env)).collect();
            let arg_vals = arg_vals?;

            // Pass the call-site name (if any) so apply_function can rebind
            // the function for recursive calls.
            let call_name = if let Expr::Symbol(name) = &**func {
                Some(name.as_str())
            } else {
                None
            };
            apply_function(&func_val, &arg_vals, env, call_name)
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
                    "list" => {
                        // Evaluate all arguments and create a list
                        let values = exprs
                            .iter()
                            .skip(1)
                            .map(|e| eval(e, env))
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(Value::List(values))
                    }
                    "map" => {
                        if exprs.len() != 3 {
                            return Err("map requires 2 arguments: (map f lst)".to_string());
                        }
                        let f = eval(&exprs[1], env)?;
                        let lst = eval(&exprs[2], env)?;
                        let items = list_items(&lst, "map")?;
                        let mut result = Vec::with_capacity(items.len());
                        for item in items {
                            result.push(apply_function(&f, &[item], env, None)?);
                        }
                        Ok(Value::List(result))
                    }
                    "filter" => {
                        if exprs.len() != 3 {
                            return Err("filter requires 2 arguments: (filter pred lst)".to_string());
                        }
                        let pred = eval(&exprs[1], env)?;
                        let lst = eval(&exprs[2], env)?;
                        let items = list_items(&lst, "filter")?;
                        let mut result = Vec::new();
                        for item in items {
                            match apply_function(&pred, std::slice::from_ref(&item), env, None)? {
                                Value::Bool(true) => result.push(item),
                                Value::Bool(false) => {}
                                other => {
                                    return Err(format!(
                                        "filter predicate must return bool, got {}",
                                        other.type_name()
                                    ))
                                }
                            }
                        }
                        if result.is_empty() {
                            Ok(Value::Nil)
                        } else {
                            Ok(Value::List(result))
                        }
                    }
                    "fold" => {
                        if exprs.len() != 4 {
                            return Err(
                                "fold requires 3 arguments: (fold f init lst)".to_string()
                            );
                        }
                        let f = eval(&exprs[1], env)?;
                        let mut acc = eval(&exprs[2], env)?;
                        let lst = eval(&exprs[3], env)?;
                        let items = list_items(&lst, "fold")?;
                        for item in items {
                            acc = apply_function(&f, &[acc, item], env, None)?;
                        }
                        Ok(acc)
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

/// Try to match `value` against `pattern`, binding any captured variables
/// into `env`. Returns true on success. On failure the caller should
/// discard `env` (bindings already written are considered scratch).
fn pattern_match(pattern: &Pattern, value: &Value, env: &mut Environment) -> bool {
    match (pattern, value) {
        (Pattern::Wildcard, _) => true,
        (Pattern::Variable(name), v) => {
            env.set(name.clone(), v.clone());
            true
        }
        (Pattern::LiteralI32(a), Value::Integer32(b)) => a == b,
        (Pattern::LiteralI64(a), Value::Integer64(b)) => a == b,
        (Pattern::LiteralF64(a), Value::Float(b)) => a == b,
        (Pattern::LiteralBool(a), Value::Bool(b)) => a == b,
        (Pattern::LiteralString(a), Value::String(b)) => a == b,
        (Pattern::Nil, Value::Nil) => true,
        (Pattern::Nil, Value::List(items)) => items.is_empty(),
        (Pattern::Cons(head_pat, tail_pat), Value::List(items)) if !items.is_empty() => {
            let head = items[0].clone();
            let tail = if items.len() == 1 {
                Value::Nil
            } else {
                Value::List(items[1..].to_vec())
            };
            pattern_match(head_pat, &head, env) && pattern_match(tail_pat, &tail, env)
        }
        // Match the inner pattern first; only bind the alias if it
        // succeeds so failed branches don't leak the alias.
        (Pattern::As(inner, name), v) if pattern_match(inner, v, env) => {
            env.set(name.clone(), v.clone());
            true
        }
        _ => false,
    }
}

/// Normalize a list-ish Value into an owned Vec<Value>.
/// `Nil` is treated as the empty list. Any other value is a type error
/// surfaced with the caller's operation name for a clear message.
fn list_items(value: &Value, op: &str) -> Result<Vec<Value>, String> {
    match value {
        Value::List(items) => Ok(items.clone()),
        Value::Nil => Ok(Vec::new()),
        other => Err(format!("{} expects a list, got {}", op, other.type_name())),
    }
}

/// Apply a function value to pre-evaluated arguments.
///
/// `call_name` is the symbol the function was looked up under at the call
/// site, if any. It's used to rebind the function into its own closure
/// environment so direct recursion works.
pub fn apply_function(
    func_val: &Value,
    args: &[Value],
    env: &Environment,
    call_name: Option<&str>,
) -> Result<Value, String> {
    match func_val {
        Value::Function { params, body, env: func_env } => {
            if params.len() != args.len() {
                return Err(format!(
                    "Wrong number of arguments: expected {}, got {}",
                    params.len(),
                    args.len()
                ));
            }

            let mut new_env = func_env.extend();

            // For recursion: if the function was called by name, make that
            // name resolvable inside the body too.
            if let Some(name) = call_name
                && let Some(func_value) = env.get(name)
            {
                new_env.set(name.to_string(), func_value.clone());
            }

            for (param, arg) in params.iter().zip(args.iter()) {
                new_env.set(param.clone(), arg.clone());
            }

            eval(body, &mut new_env)
        }
        Value::BuiltinFunction { arity, func, name } => {
            if args.len() != *arity {
                return Err(format!(
                    "Wrong number of arguments for {}: expected {}, got {}",
                    name, arity, args.len()
                ));
            }
            func(args)
        }
        _ => Err(format!("Cannot call non-function value: {}", func_val)),
    }
}

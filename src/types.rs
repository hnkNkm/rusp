use crate::ast::{Expr, Type};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypeEnv {
    types: HashMap<String, Type>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut types = HashMap::new();
        
        // i32 arithmetic operators
        types.insert("+".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Inferred),
        });
        types.insert("-".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Inferred),
        });
        types.insert("*".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Inferred),
        });
        types.insert("/".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Inferred),
        });
        
        types.insert("+.".to_string(), Type::Function {
            params: vec![Type::F64, Type::F64],
            return_type: Box::new(Type::F64),
        });
        types.insert("-.".to_string(), Type::Function {
            params: vec![Type::F64, Type::F64],
            return_type: Box::new(Type::F64),
        });
        types.insert("*.".to_string(), Type::Function {
            params: vec![Type::F64, Type::F64],
            return_type: Box::new(Type::F64),
        });
        types.insert("/.".to_string(), Type::Function {
            params: vec![Type::F64, Type::F64],
            return_type: Box::new(Type::F64),
        });
        
        types.insert("=".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Bool),
        });
        types.insert("<".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Bool),
        });
        types.insert(">".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Bool),
        });
        types.insert("<=".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Bool),
        });
        types.insert(">=".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::Inferred],
            return_type: Box::new(Type::Bool),
        });
        
        types.insert("and".to_string(), Type::Function {
            params: vec![Type::Bool, Type::Bool],
            return_type: Box::new(Type::Bool),
        });
        types.insert("or".to_string(), Type::Function {
            params: vec![Type::Bool, Type::Bool],
            return_type: Box::new(Type::Bool),
        });
        types.insert("not".to_string(), Type::Function {
            params: vec![Type::Bool],
            return_type: Box::new(Type::Bool),
        });
        
        // print and println can accept any type
        // We use Inferred to represent "any type" for now
        types.insert("print".to_string(), Type::Function {
            params: vec![Type::Inferred],
            return_type: Box::new(Type::Inferred),
        });
        types.insert("println".to_string(), Type::Function {
            params: vec![Type::Inferred],
            return_type: Box::new(Type::Inferred),
        });
        
        // List operations
        types.insert("cons".to_string(), Type::Function {
            params: vec![Type::Inferred, Type::List(Box::new(Type::Inferred))],
            return_type: Box::new(Type::List(Box::new(Type::Inferred))),
        });
        types.insert("car".to_string(), Type::Function {
            params: vec![Type::List(Box::new(Type::Inferred))],
            return_type: Box::new(Type::Inferred),
        });
        types.insert("cdr".to_string(), Type::Function {
            params: vec![Type::List(Box::new(Type::Inferred))],
            return_type: Box::new(Type::List(Box::new(Type::Inferred))),
        });
        types.insert("null?".to_string(), Type::Function {
            params: vec![Type::Inferred],
            return_type: Box::new(Type::Bool),
        });
        types.insert("length".to_string(), Type::Function {
            params: vec![Type::List(Box::new(Type::Inferred))],
            return_type: Box::new(Type::I32),
        });
        types.insert("append".to_string(), Type::Function {
            params: vec![Type::List(Box::new(Type::Inferred)), Type::List(Box::new(Type::Inferred))],
            return_type: Box::new(Type::List(Box::new(Type::Inferred))),
        });
        types.insert("nth".to_string(), Type::Function {
            params: vec![Type::I32, Type::List(Box::new(Type::Inferred))],
            return_type: Box::new(Type::Inferred),
        });
        
        TypeEnv { types }
    }
    
    pub fn get(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }
    
    pub fn insert(&mut self, name: String, ty: Type) {
        self.types.insert(name, ty);
    }
    
    pub fn extend(&self) -> Self {
        TypeEnv {
            types: self.types.clone(),
        }
    }
}

pub fn type_check(expr: &Expr, env: &mut TypeEnv) -> Result<Type, String> {
    match expr {
        Expr::Integer32(_) => Ok(Type::I32),
        Expr::Integer64(_) => Ok(Type::I64),
        Expr::Float(_) => Ok(Type::F64),
        Expr::Bool(_) => Ok(Type::Bool),
        Expr::String(_) => Ok(Type::String),
        Expr::Nil => Ok(Type::List(Box::new(Type::Inferred))),

        Expr::Symbol(name) => {
            env.get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable: {}", name))
        }
        
        Expr::If { condition, then_branch, else_branch } => {
            let cond_type = type_check(condition, env)?;
            if cond_type != Type::Bool {
                return Err(format!("If condition must be bool, got {}", cond_type));
            }
            
            let then_type = type_check(then_branch, env)?;
            let else_type = type_check(else_branch, env)?;
            
            // Use types_match for more flexible type checking
            if !types_match(&then_type, &else_type) {
                return Err(format!(
                    "If branches must have same type: {} vs {}",
                    then_type, else_type
                ));
            }
            
            // Return the more specific type
            Ok(if then_type == Type::List(Box::new(Type::Inferred)) && else_type != Type::List(Box::new(Type::Inferred)) {
                else_type
            } else {
                then_type
            })
        }
        
        Expr::Let { name, type_ann, value, body } => {
            let value_type = type_check(value, env)?;
            
            let binding_type = if let Some(ann) = type_ann {
                if ann != &value_type && ann != &Type::Inferred {
                    return Err(format!(
                        "Type mismatch: expected {}, got {}",
                        ann, value_type
                    ));
                }
                ann.clone()
            } else {
                value_type
            };
            
            if let Some(body_expr) = body {
                // Let-in expression: type check body in new scope
                let mut new_env = env.extend();
                new_env.insert(name.clone(), binding_type);
                type_check(body_expr, &mut new_env)
            } else {
                // Simple let: add to current environment
                env.insert(name.clone(), binding_type.clone());
                Ok(binding_type)
            }
        }
        
        Expr::Defn { name, params, return_type, body } => {
            // First, add the function type to the environment for recursion
            let func_type = Type::Function {
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                return_type: Box::new(return_type.clone()),
            };
            env.insert(name.clone(), func_type.clone());
            
            // Now type-check the body with the function in scope
            let mut new_env = env.extend();
            
            for (param_name, param_type) in params {
                new_env.insert(param_name.clone(), param_type.clone());
            }
            
            let body_type = type_check(body, &mut new_env)?;
            
            if &body_type != return_type && return_type != &Type::Inferred {
                return Err(format!(
                    "Return type mismatch: expected {}, got {}",
                    return_type, body_type
                ));
            }
            
            Ok(func_type)
        }
        
        Expr::Lambda { params, return_type, body } => {
            let mut new_env = env.extend();
            
            for (param_name, param_type) in params {
                new_env.insert(param_name.clone(), param_type.clone());
            }
            
            let body_type = type_check(body, &mut new_env)?;
            
            if let Some(rt) = return_type {
                if &body_type != rt && rt != &Type::Inferred {
                    return Err(format!(
                        "Lambda return type mismatch: expected {}, got {}",
                        rt, body_type
                    ));
                }
            }
            
            Ok(Type::Function {
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                return_type: Box::new(body_type),
            })
        }
        
        Expr::Call { func, args } => {
            let func_type = type_check(func, env)?;
            
            match func_type {
                Type::Function { params, return_type } => {
                    if args.len() != params.len() {
                        return Err(format!(
                            "Wrong number of arguments: expected {}, got {}",
                            params.len(), args.len()
                        ));
                    }
                    
                    let mut actual_return_type = *return_type.clone();
                    
                    for (i, (arg, param_type)) in args.iter().zip(params.iter()).enumerate() {
                        let arg_type = type_check(arg, env)?;
                        // Check type compatibility
                        if !types_match(param_type, &arg_type) {
                            return Err(format!(
                                "Type mismatch in argument: expected {}, got {}",
                                param_type, arg_type
                            ));
                        }
                        // Special handling for list operations
                        if let Expr::Symbol(fname) = &**func {
                            match fname.as_str() {
                                "car" => {
                                    // car returns the element type of the list
                                    if let Type::List(elem_type) = &arg_type {
                                        actual_return_type = *elem_type.clone();
                                    }
                                }
                                "cons" => {
                                    // cons returns a list of the first argument's type
                                    if i == 0 {
                                        actual_return_type = Type::List(Box::new(arg_type.clone()));
                                    }
                                }
                                "cdr" | "append" => {
                                    // cdr/append preserve the list type of their argument
                                    if let Type::List(_) = &arg_type {
                                        actual_return_type = arg_type.clone();
                                    }
                                }
                                "nth" => {
                                    // nth returns the element type of the list (second arg)
                                    if i == 1 {
                                        if let Type::List(elem_type) = &arg_type {
                                            actual_return_type = *elem_type.clone();
                                        }
                                    }
                                }
                                _ => {
                                    // If the function returns Inferred, return the actual argument type
                                    if *return_type == Type::Inferred {
                                        actual_return_type = arg_type;
                                    }
                                }
                            }
                        } else if *return_type == Type::Inferred {
                            actual_return_type = arg_type;
                        }
                    }
                    
                    Ok(actual_return_type)
                }
                _ => Err(format!("Cannot call non-function type: {}", func_type)),
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
                        type_check(&Expr::If {
                            condition: Box::new(exprs[1].clone()),
                            then_branch: Box::new(exprs[2].clone()),
                            else_branch: Box::new(exprs[3].clone()),
                        }, env)
                    }
                    "list" => {
                        // Empty list: (list) -> List<_>
                        if exprs.len() == 1 {
                            return Ok(Type::List(Box::new(Type::Inferred)));
                        }

                        // Require all elements to share a type with the first.
                        // types_match allows Inferred on either side so
                        // `(list 1 nil)` style mixing with unresolved types
                        // still works where appropriate.
                        let first_type = type_check(&exprs[1], env)?;
                        for (offset, elem) in exprs.iter().enumerate().skip(2) {
                            let elem_type = type_check(elem, env)?;
                            if !types_match(&first_type, &elem_type) {
                                return Err(format!(
                                    "List element type mismatch at position {}: expected {}, got {}",
                                    offset - 1,
                                    first_type,
                                    elem_type
                                ));
                            }
                        }
                        Ok(Type::List(Box::new(first_type)))
                    }
                    "map" => {
                        // (map f lst) : List<B> where f : A -> B and lst : List<A>
                        if exprs.len() != 3 {
                            return Err("map requires 2 arguments: (map f lst)".to_string());
                        }
                        let f_type = type_check(&exprs[1], env)?;
                        let lst_type = type_check(&exprs[2], env)?;
                        let elem_type = expect_list_elem(&lst_type, "map")?;
                        let (param_types, ret_type) = expect_function(&f_type, "map")?;
                        if param_types.len() != 1 {
                            return Err(format!(
                                "map requires a unary function, got arity {}",
                                param_types.len()
                            ));
                        }
                        if !types_match(&param_types[0], &elem_type) {
                            return Err(format!(
                                "map function parameter type {} does not match list element type {}",
                                param_types[0], elem_type
                            ));
                        }
                        // If the function's return type is unresolved, the list
                        // element type is the best guess we have.
                        let result_elem = if ret_type == Type::Inferred {
                            elem_type
                        } else {
                            ret_type
                        };
                        Ok(Type::List(Box::new(result_elem)))
                    }
                    "filter" => {
                        // (filter pred lst) : List<A> where pred : A -> bool
                        if exprs.len() != 3 {
                            return Err(
                                "filter requires 2 arguments: (filter pred lst)".to_string()
                            );
                        }
                        let pred_type = type_check(&exprs[1], env)?;
                        let lst_type = type_check(&exprs[2], env)?;
                        let elem_type = expect_list_elem(&lst_type, "filter")?;
                        let (param_types, ret_type) = expect_function(&pred_type, "filter")?;
                        if param_types.len() != 1 {
                            return Err(format!(
                                "filter requires a unary predicate, got arity {}",
                                param_types.len()
                            ));
                        }
                        if !types_match(&param_types[0], &elem_type) {
                            return Err(format!(
                                "filter predicate parameter type {} does not match list element type {}",
                                param_types[0], elem_type
                            ));
                        }
                        if !types_match(&ret_type, &Type::Bool) {
                            return Err(format!(
                                "filter predicate must return bool, got {}",
                                ret_type
                            ));
                        }
                        Ok(Type::List(Box::new(elem_type)))
                    }
                    "fold" => {
                        // (fold f init lst) : B where f : B -> A -> B, init : B, lst : List<A>
                        if exprs.len() != 4 {
                            return Err(
                                "fold requires 3 arguments: (fold f init lst)".to_string()
                            );
                        }
                        let f_type = type_check(&exprs[1], env)?;
                        let init_type = type_check(&exprs[2], env)?;
                        let lst_type = type_check(&exprs[3], env)?;
                        let elem_type = expect_list_elem(&lst_type, "fold")?;
                        let (param_types, ret_type) = expect_function(&f_type, "fold")?;
                        if param_types.len() != 2 {
                            return Err(format!(
                                "fold requires a binary function, got arity {}",
                                param_types.len()
                            ));
                        }
                        if !types_match(&param_types[0], &init_type) {
                            return Err(format!(
                                "fold accumulator type {} does not match init type {}",
                                param_types[0], init_type
                            ));
                        }
                        if !types_match(&param_types[1], &elem_type) {
                            return Err(format!(
                                "fold element parameter type {} does not match list element type {}",
                                param_types[1], elem_type
                            ));
                        }
                        if !types_match(&ret_type, &init_type) {
                            return Err(format!(
                                "fold return type {} does not match accumulator type {}",
                                ret_type, init_type
                            ));
                        }
                        // Prefer the concrete init type over any Inferred from
                        // the function's return slot.
                        Ok(init_type)
                    }
                    "let" => {
                        if exprs.len() < 3 {
                            return Err("Let requires at least 2 arguments".to_string());
                        }
                        
                        if let Expr::Symbol(name) = &exprs[1] {
                            let (type_ann, value_idx, body) = if exprs.len() == 4 {
                                // Could be (let name type value) or (let name value body)
                                if let Expr::Symbol(ty_str) = &exprs[2] {
                                    if parse_type(ty_str).is_ok() {
                                        let ty = parse_type(ty_str)?;
                                        (Some(ty), 3, None)
                                    } else {
                                        // Not a type, treat as (let name value body)
                                        (None, 2, Some(Box::new(exprs[3].clone())))
                                    }
                                } else {
                                    // Not a type symbol, treat as (let name value body)
                                    (None, 2, Some(Box::new(exprs[3].clone())))
                                }
                            } else if exprs.len() == 5 {
                                // (let name type value body)
                                if let Expr::Symbol(ty_str) = &exprs[2] {
                                    let ty = parse_type(ty_str)?;
                                    (Some(ty), 3, Some(Box::new(exprs[4].clone())))
                                } else {
                                    return Err("Invalid type annotation".to_string());
                                }
                            } else {
                                (None, 2, None)
                            };
                            
                            type_check(&Expr::Let {
                                name: name.clone(),
                                type_ann,
                                value: Box::new(exprs[value_idx].clone()),
                                body,
                            }, env)
                        } else {
                            Err("Let binding must have a symbol name".to_string())
                        }
                    }
                    _ => {
                        type_check(&Expr::Call {
                            func: Box::new(exprs[0].clone()),
                            args: exprs[1..].to_vec(),
                        }, env)
                    }
                }
            } else {
                type_check(&Expr::Call {
                    func: Box::new(exprs[0].clone()),
                    args: exprs[1..].to_vec(),
                }, env)
            }
        }
    }
}

pub fn parse_type(s: &str) -> Result<Type, String> {
    match s {
        "i32" => Ok(Type::I32),
        "i64" => Ok(Type::I64),
        "f64" => Ok(Type::F64),
        "bool" => Ok(Type::Bool),
        "String" => Ok(Type::String),
        "_" => Ok(Type::Inferred),
        _ => Err(format!("Unknown type: {}", s)),
    }
}

/// Unwrap a `List<T>` type to its element type, or normalize `Nil`-shaped
/// cases. Returns an error naming the offending operation for clarity.
fn expect_list_elem(ty: &Type, op: &str) -> Result<Type, String> {
    match ty {
        Type::List(elem) => Ok(*elem.clone()),
        _ => Err(format!("{} expects a list, got {}", op, ty)),
    }
}

/// Unwrap a `Function` type, returning `(params, return_type)`.
fn expect_function(ty: &Type, op: &str) -> Result<(Vec<Type>, Type), String> {
    match ty {
        Type::Function { params, return_type } => Ok((params.clone(), *return_type.clone())),
        _ => Err(format!("{} expects a function, got {}", op, ty)),
    }
}

fn types_match(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        // Inferred matches anything
        (Type::Inferred, _) | (_, Type::Inferred) => true,
        
        // List types match if element types match
        (Type::List(e1), Type::List(e2)) => types_match(e1, e2),
        
        // Function types match if params and return match
        (Type::Function { params: p1, return_type: r1 }, 
         Type::Function { params: p2, return_type: r2 }) => {
            p1.len() == p2.len() && 
            p1.iter().zip(p2.iter()).all(|(a, b)| types_match(a, b)) &&
            types_match(r1, r2)
        }
        
        // Exact match
        (t1, t2) => t1 == t2,
    }
}
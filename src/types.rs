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
            
            if then_type != else_type {
                return Err(format!(
                    "If branches must have same type: {} vs {}",
                    then_type, else_type
                ));
            }
            
            Ok(then_type)
        }
        
        Expr::Let { name, type_ann, value } => {
            let value_type = type_check(value, env)?;
            
            if let Some(ann) = type_ann {
                if ann != &value_type && ann != &Type::Inferred {
                    return Err(format!(
                        "Type mismatch: expected {}, got {}",
                        ann, value_type
                    ));
                }
                env.insert(name.clone(), ann.clone());
                Ok(ann.clone())
            } else {
                env.insert(name.clone(), value_type.clone());
                Ok(value_type)
            }
        }
        
        Expr::Defn { name, params, return_type, body } => {
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
            
            let func_type = Type::Function {
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                return_type: Box::new(return_type.clone()),
            };
            
            env.insert(name.clone(), func_type.clone());
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
                    
                    for (arg, param_type) in args.iter().zip(params.iter()) {
                        let arg_type = type_check(arg, env)?;
                        // Inferred type can match any type (for print/println)
                        if *param_type != Type::Inferred && arg_type != *param_type {
                            return Err(format!(
                                "Type mismatch in argument: expected {}, got {}",
                                param_type, arg_type
                            ));
                        }
                        // If the function returns Inferred, return the actual argument type
                        if *return_type == Type::Inferred {
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
                    "let" => {
                        if exprs.len() < 3 {
                            return Err("Let requires at least 2 arguments".to_string());
                        }
                        
                        if let Expr::Symbol(name) = &exprs[1] {
                            let (type_ann, value_idx) = if exprs.len() == 4 {
                                if let Expr::Symbol(ty_str) = &exprs[2] {
                                    let ty = parse_type(ty_str)?;
                                    (Some(ty), 3)
                                } else {
                                    return Err("Invalid type annotation".to_string());
                                }
                            } else {
                                (None, 2)
                            };
                            
                            type_check(&Expr::Let {
                                name: name.clone(),
                                type_ann,
                                value: Box::new(exprs[value_idx].clone()),
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
        "f64" => Ok(Type::F64),
        "bool" => Ok(Type::Bool),
        "String" => Ok(Type::String),
        "_" => Ok(Type::Inferred),
        _ => Err(format!("Unknown type: {}", s)),
    }
}
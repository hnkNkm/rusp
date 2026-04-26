use crate::ast::{Expr, Pattern, Type};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypeEnv {
    types: HashMap<String, Type>,
    /// Bidirectional inference (段階 A): records the concrete types into
    /// which `Inferred` parameters were narrowed during body type-checking.
    /// `Defn` reads this back to refine its registered function signature
    /// after the body is checked.
    pub refinements: HashMap<String, Type>,
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
        
        TypeEnv { types, refinements: HashMap::new() }
    }

    pub fn get(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }

    pub fn insert(&mut self, name: String, ty: Type) {
        self.types.insert(name, ty);
    }

    pub fn extend(&self) -> Self {
        // Child scope inherits known types but starts with a fresh
        // refinements map. Each function body is its own refinement
        // scope; `Defn` lifts only the params it cares about.
        TypeEnv {
            types: self.types.clone(),
            refinements: HashMap::new(),
        }
    }

    /// Bidirectional inference (段階 A): narrow `name`'s type to `ty`.
    ///
    /// - If `name` is unknown, no-op (only call this for known variables).
    /// - If `name` is currently `Inferred`, replace with `ty`.
    /// - If `name` is currently `List<Inferred>` and `ty` is a more concrete
    ///   list type, replace with `ty`.
    /// - If `name` is already concrete and matches `ty`, no-op.
    /// - Otherwise, return a conflict error.
    pub fn refine(&mut self, name: &str, ty: Type) -> Result<(), String> {
        let current = match self.types.get(name) {
            Some(c) => c.clone(),
            None => return Ok(()),
        };
        match current {
            Type::Inferred => {
                self.types.insert(name.to_string(), ty.clone());
                self.refinements.insert(name.to_string(), ty);
                Ok(())
            }
            Type::List(ref inner) if matches!(**inner, Type::Inferred) => {
                // `List<_>` accepts narrowing to a more concrete list type.
                if matches!(ty, Type::List(_)) {
                    self.types.insert(name.to_string(), ty.clone());
                    self.refinements.insert(name.to_string(), ty);
                    Ok(())
                } else if types_match(&current, &ty) {
                    Ok(())
                } else {
                    Err(format!(
                        "parameter `{}` was previously {} but now requires {}",
                        name, current, ty
                    ))
                }
            }
            existing => {
                if types_match(&existing, &ty) {
                    Ok(())
                } else {
                    Err(format!(
                        "parameter `{}` was previously {} but now requires {}",
                        name, existing, ty
                    ))
                }
            }
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

            if !types_match(&body_type, return_type) && return_type != &Type::Inferred {
                return Err(format!(
                    "Return type mismatch: expected {}, got {}",
                    return_type, body_type
                ));
            }

            // Bidirectional inference (段階 A): if any `_` parameters were
            // narrowed during body type-checking, reflect them in the
            // signature so external callers see the precise type.
            let refined_params: Vec<Type> = params
                .iter()
                .map(|(pname, ptype)| {
                    new_env
                        .refinements
                        .get(pname)
                        .cloned()
                        .unwrap_or_else(|| ptype.clone())
                })
                .collect();
            let refined_func_type = Type::Function {
                params: refined_params,
                return_type: Box::new(return_type.clone()),
            };
            env.insert(name.clone(), refined_func_type.clone());

            Ok(refined_func_type)
        }
        
        Expr::Lambda { params, return_type, body } => {
            let mut new_env = env.extend();
            
            for (param_name, param_type) in params {
                new_env.insert(param_name.clone(), param_type.clone());
            }
            
            let body_type = type_check(body, &mut new_env)?;
            
            if let Some(rt) = return_type
                && &body_type != rt
                && rt != &Type::Inferred
            {
                return Err(format!(
                    "Lambda return type mismatch: expected {}, got {}",
                    rt, body_type
                ));
            }
            
            Ok(Type::Function {
                params: params.iter().map(|(_, t)| t.clone()).collect(),
                return_type: Box::new(body_type),
            })
        }
        
        Expr::Match { scrutinee, arms } => {
            let mut scrutinee_type = type_check(scrutinee, env)?;

            // Bidirectional inference (段階 A): if the scrutinee is a plain
            // variable that we still see as Inferred, but the arms reveal
            // structural usage (cons or nil), narrow it to `List<_>` so the
            // per-arm type checks and exhaustiveness see a real list type.
            if matches!(scrutinee_type, Type::Inferred)
                && let Expr::Symbol(sym) = &**scrutinee
                && arms.iter().any(|(p, _)| is_list_shaped(p))
            {
                env.refine(sym, Type::List(Box::new(Type::Inferred)))?;
                scrutinee_type = Type::List(Box::new(Type::Inferred));
            }

            // Validate each arm. Bindings introduced by the pattern are
            // visible only in that arm's body — we clone the env so
            // sibling arms don't see them.
            let mut result_type: Option<Type> = None;
            for (pat, body) in arms {
                check_pattern(pat, &scrutinee_type, env)?;
                let mut arm_env = env.extend();
                bind_pattern(pat, &scrutinee_type, &mut arm_env);
                let body_type = type_check(body, &mut arm_env)?;
                match &result_type {
                    None => result_type = Some(body_type),
                    Some(expected) => {
                        if !types_match(expected, &body_type) {
                            return Err(format!(
                                "match arms must have the same type: {} vs {}",
                                expected, body_type
                            ));
                        }
                    }
                }
            }

            // Exhaustiveness check after per-arm type checking, so per-arm
            // type errors take precedence over a less-specific exhaustiveness
            // message.
            let arm_pats: Vec<&Pattern> = arms.iter().map(|(p, _)| p).collect();
            crate::exhaustiveness::check(&scrutinee_type, &arm_pats)?;

            // Parser guarantees at least one arm, but be defensive.
            result_type.ok_or_else(|| "match has no arms".to_string())
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
                        // Bidirectional inference (段階 A): if the parameter
                        // expects a list (any list, possibly `List<_>`) and
                        // the argument is an `Inferred` symbol, narrow that
                        // symbol to `List<_>` so subsequent uses see a list.
                        if matches!(arg_type, Type::Inferred)
                            && matches!(param_type, Type::List(_))
                            && let Expr::Symbol(sym) = arg
                        {
                            env.refine(sym, Type::List(Box::new(Type::Inferred)))?;
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
                                    if i == 1
                                        && let Type::List(elem_type) = &arg_type
                                    {
                                        actual_return_type = *elem_type.clone();
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
                        // Bidirectional inference (段階 A): narrow the source
                        // variable when the list type is still Inferred but
                        // the lambda's parameter type is concrete.
                        if matches!(lst_type, Type::Inferred)
                            && let Expr::Symbol(sym) = &exprs[2]
                            && !matches!(param_types[0], Type::Inferred)
                        {
                            env.refine(sym, Type::List(Box::new(param_types[0].clone())))?;
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
                        // Bidirectional inference (段階 A): narrow the source
                        // variable from the predicate's parameter type.
                        if matches!(lst_type, Type::Inferred)
                            && let Expr::Symbol(sym) = &exprs[2]
                            && !matches!(param_types[0], Type::Inferred)
                        {
                            env.refine(sym, Type::List(Box::new(param_types[0].clone())))?;
                        }
                        let result_elem = if matches!(elem_type, Type::Inferred)
                            && !matches!(param_types[0], Type::Inferred)
                        {
                            param_types[0].clone()
                        } else {
                            elem_type
                        };
                        Ok(Type::List(Box::new(result_elem)))
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
                        // Bidirectional inference (段階 A): narrow the source
                        // variable from the lambda's element-parameter type.
                        if matches!(lst_type, Type::Inferred)
                            && let Expr::Symbol(sym) = &exprs[3]
                            && !matches!(param_types[1], Type::Inferred)
                        {
                            env.refine(sym, Type::List(Box::new(param_types[1].clone())))?;
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
        // Bidirectional inference (段階 A): an unresolved scrutinee is
        // accepted here. Caller will narrow the source variable via
        // `TypeEnv::refine` once the function/lambda parameter types are
        // known.
        Type::Inferred => Ok(Type::Inferred),
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

/// Strip `As` wrappers and report whether the underlying pattern is a
/// list-shaped constructor (`cons` or `nil`). Used by the bidirectional
/// scrutinee refinement in `Match` to decide when an `Inferred` scrutinee
/// can be narrowed to `List<_>`.
fn is_list_shaped(pat: &Pattern) -> bool {
    match pat {
        Pattern::Cons(_, _) | Pattern::Nil => true,
        Pattern::As(inner, _) => is_list_shaped(inner),
        Pattern::Guard(inner, _) => is_list_shaped(inner),
        // Any branch hinting at a list structure is enough to refine
        // the scrutinee to `List<_>`. Mismatched branches (e.g. a literal
        // i32 alongside a cons) will still be reported by check_pattern.
        Pattern::Or(branches) => branches.iter().any(is_list_shaped),
        _ => false,
    }
}

/// Pure version of `bind_pattern`: returns the (name, type) bindings that
/// the pattern would introduce, without mutating any environment. Used for
/// or-pattern soundness — every branch must produce the same set of
/// bindings (same names, compatible types).
fn collect_bindings(
    pat: &Pattern,
    scrutinee: &Type,
) -> Result<HashMap<String, Type>, String> {
    match pat {
        Pattern::Wildcard
        | Pattern::Nil
        | Pattern::LiteralI32(_)
        | Pattern::LiteralI64(_)
        | Pattern::LiteralF64(_)
        | Pattern::LiteralBool(_)
        | Pattern::LiteralString(_) => Ok(HashMap::new()),
        Pattern::Variable(name) => {
            let mut m = HashMap::new();
            m.insert(name.clone(), scrutinee.clone());
            Ok(m)
        }
        Pattern::Cons(head, tail) => {
            let (head_ty, tail_ty) = match scrutinee {
                Type::List(elem) => (*elem.clone(), scrutinee.clone()),
                _ => (Type::Inferred, Type::List(Box::new(Type::Inferred))),
            };
            let mut m = collect_bindings(head, &head_ty)?;
            for (k, v) in collect_bindings(tail, &tail_ty)? {
                m.insert(k, v);
            }
            Ok(m)
        }
        Pattern::As(inner, name) => {
            let mut m = collect_bindings(inner, scrutinee)?;
            m.insert(name.clone(), scrutinee.clone());
            Ok(m)
        }
        Pattern::Guard(inner, _) => collect_bindings(inner, scrutinee),
        Pattern::Or(branches) => {
            // Defensive: parser rejects empty or, but guard the invariant.
            if branches.is_empty() {
                return Err("empty or-pattern".to_string());
            }
            let first = collect_bindings(&branches[0], scrutinee)?;
            for b in &branches[1..] {
                let m = collect_bindings(b, scrutinee)?;
                // Same key sets in both directions.
                for k in first.keys() {
                    if !m.contains_key(k) {
                        return Err(format!(
                            "or-pattern: variable `{}` bound in some branches but not others",
                            k
                        ));
                    }
                }
                for (k, v) in &m {
                    match first.get(k) {
                        None => {
                            return Err(format!(
                                "or-pattern: variable `{}` bound in some branches but not others",
                                k
                            ));
                        }
                        Some(expected) if !types_match(expected, v) => {
                            return Err(format!(
                                "or-pattern: variable `{}` has inconsistent types: {} vs {}",
                                k, expected, v
                            ));
                        }
                        Some(_) => {}
                    }
                }
            }
            Ok(first)
        }
    }
}

/// Validate that `pattern` is compatible with `scrutinee` type.
///
/// Wildcards and variables match anything. Literal patterns must match
/// their primitive type. `nil` and `cons` require the scrutinee to be
/// a list type (or Inferred). `guard` requires its expression to be
/// `bool` when type-checked with the inner pattern's bindings in scope —
/// hence the `env` parameter.
fn check_pattern(
    pattern: &Pattern,
    scrutinee: &Type,
    env: &mut TypeEnv,
) -> Result<(), String> {
    match pattern {
        Pattern::Wildcard | Pattern::Variable(_) => Ok(()),
        Pattern::LiteralI32(_) => {
            if types_match(scrutinee, &Type::I32) {
                Ok(())
            } else {
                Err(format!("pattern i32 does not match scrutinee type {}", scrutinee))
            }
        }
        Pattern::LiteralI64(_) => {
            if types_match(scrutinee, &Type::I64) {
                Ok(())
            } else {
                Err(format!("pattern i64 does not match scrutinee type {}", scrutinee))
            }
        }
        Pattern::LiteralF64(_) => {
            if types_match(scrutinee, &Type::F64) {
                Ok(())
            } else {
                Err(format!("pattern f64 does not match scrutinee type {}", scrutinee))
            }
        }
        Pattern::LiteralBool(_) => {
            if types_match(scrutinee, &Type::Bool) {
                Ok(())
            } else {
                Err(format!("pattern bool does not match scrutinee type {}", scrutinee))
            }
        }
        Pattern::LiteralString(_) => {
            if types_match(scrutinee, &Type::String) {
                Ok(())
            } else {
                Err(format!("pattern String does not match scrutinee type {}", scrutinee))
            }
        }
        Pattern::Nil => match scrutinee {
            Type::List(_) | Type::Inferred => Ok(()),
            _ => Err(format!("nil pattern requires a list, got {}", scrutinee)),
        },
        Pattern::Cons(head, tail) => match scrutinee {
            Type::List(elem) => {
                check_pattern(head, elem, env)?;
                check_pattern(tail, scrutinee, env)?;
                Ok(())
            }
            // After bidirectional refinement (段階 A), an `Inferred`
            // scrutinee should have been narrowed to `List<_>` before we
            // get here. Surface a defensive internal error if not — this
            // signals a missing refinement site rather than a user bug.
            Type::Inferred => Err(
                "internal: cons pattern reached Inferred scrutinee — should have been refined".to_string()
            ),
            _ => Err(format!("cons pattern requires a list, got {}", scrutinee)),
        },
        Pattern::As(inner, _) => check_pattern(inner, scrutinee, env),
        Pattern::Guard(inner, guard_expr) => {
            // Inner pattern must itself be valid against the scrutinee type.
            check_pattern(inner, scrutinee, env)?;
            // The guard expression sees the inner pattern's bindings, so
            // type-check it in an extended env. Bindings are scoped to the
            // guard check and the arm body (the caller already extends env
            // for the body, so we don't pollute its env here).
            let mut guard_env = env.extend();
            bind_pattern(inner, scrutinee, &mut guard_env);
            let ty = type_check(guard_expr, &mut guard_env)?;
            if !types_match(&ty, &Type::Bool) {
                return Err(format!("guard expression must be Bool, got {}", ty));
            }
            Ok(())
        }
        Pattern::Or(branches) => {
            if branches.is_empty() {
                return Err("empty or-pattern".to_string());
            }
            // Each branch must be type-compatible with the scrutinee.
            for b in branches {
                check_pattern(b, scrutinee, env)?;
            }
            // All branches must introduce the same (name, type) bindings.
            collect_bindings(pattern, scrutinee).map(|_| ())
        }
    }
}

/// Bind any variables introduced by `pattern` into `env` with appropriate
/// types derived from `scrutinee`. Caller is responsible for scoping
/// (typically via `env.extend()`).
fn bind_pattern(pattern: &Pattern, scrutinee: &Type, env: &mut TypeEnv) {
    match pattern {
        Pattern::Wildcard
        | Pattern::LiteralI32(_)
        | Pattern::LiteralI64(_)
        | Pattern::LiteralF64(_)
        | Pattern::LiteralBool(_)
        | Pattern::LiteralString(_)
        | Pattern::Nil => {}
        Pattern::Variable(name) => {
            env.insert(name.clone(), scrutinee.clone());
        }
        Pattern::Cons(head, tail) => {
            let (head_ty, tail_ty) = match scrutinee {
                Type::List(elem) => (*elem.clone(), scrutinee.clone()),
                _ => (Type::Inferred, Type::List(Box::new(Type::Inferred))),
            };
            bind_pattern(head, &head_ty, env);
            bind_pattern(tail, &tail_ty, env);
        }
        Pattern::As(inner, name) => {
            // Alias gets the whole scrutinee; the inner pattern adds any
            // sub-bindings on top.
            env.insert(name.clone(), scrutinee.clone());
            bind_pattern(inner, scrutinee, env);
        }
        Pattern::Guard(inner, _) => {
            // Guard expr does not introduce bindings; the inner pattern does.
            bind_pattern(inner, scrutinee, env);
        }
        Pattern::Or(branches) => {
            // `check_pattern` ensured all branches introduce the same
            // (name, type) bindings, so binding from the first branch is
            // sufficient for type-checking the arm body.
            if let Some(first) = branches.first() {
                bind_pattern(first, scrutinee, env);
            }
        }
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
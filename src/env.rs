use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Integer32(i32),
    Integer64(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Function {
        params: Vec<String>,
        body: crate::ast::Expr,
        env: Environment,
    },
    BuiltinFunction {
        name: String,
        arity: usize,
        func: fn(&[Value]) -> Result<Value, String>,
    },
    List(Vec<Value>),  // List value
    Nil,               // Empty list / nil
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Integer32(n) => write!(f, "{}", n),
            Value::Integer64(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Function { params, .. } => {
                write!(f, "#<function:{}>", params.len())
            }
            Value::BuiltinFunction { name, arity, .. } => {
                write!(f, "#<builtin:{}:{}>", name, arity)
            }
            Value::List(values) => {
                write!(f, "(")?;
                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }
            Value::Nil => write!(f, "nil"),
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::Integer32(_) => "i32",
            Value::Integer64(_) => "i64",
            Value::Float(_) => "f64",
            Value::Bool(_) => "bool",
            Value::String(_) => "String",
            Value::Function { .. } => "function",
            Value::BuiltinFunction { .. } => "builtin",
            Value::List(_) => "list",
            Value::Nil => "nil",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Environment {
            values: HashMap::new(),
            parent: None,
        };
        
        env.values.insert("+".to_string(), Value::BuiltinFunction {
            name: "+".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Integer32(a + b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Integer64(a + b)),
                    _ => Err("+ requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("-".to_string(), Value::BuiltinFunction {
            name: "-".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Integer32(a - b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Integer64(a - b)),
                    _ => Err("- requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("*".to_string(), Value::BuiltinFunction {
            name: "*".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Integer32(a * b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Integer64(a * b)),
                    _ => Err("* requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("/".to_string(), Value::BuiltinFunction {
            name: "/".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => {
                        if *b == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Integer32(a / b))
                        }
                    }
                    (Value::Integer64(a), Value::Integer64(b)) => {
                        if *b == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Integer64(a / b))
                        }
                    }
                    _ => Err("/ requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("+.".to_string(), Value::BuiltinFunction {
            name: "+.".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                    _ => Err("+. requires two floats".to_string()),
                }
            },
        });
        
        env.values.insert("-.".to_string(), Value::BuiltinFunction {
            name: "-.".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                    _ => Err("-. requires two floats".to_string()),
                }
            },
        });
        
        env.values.insert("*.".to_string(), Value::BuiltinFunction {
            name: "*.".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                    _ => Err("*. requires two floats".to_string()),
                }
            },
        });
        
        env.values.insert("/.".to_string(), Value::BuiltinFunction {
            name: "/.".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Float(a), Value::Float(b)) => {
                        if *b == 0.0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Float(a / b))
                        }
                    }
                    _ => Err("/. requires two floats".to_string()),
                }
            },
        });
        
        env.values.insert("=".to_string(), Value::BuiltinFunction {
            name: "=".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Bool(a == b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Bool(a == b)),
                    _ => Err("= requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("<".to_string(), Value::BuiltinFunction {
            name: "<".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Bool(a < b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Bool(a < b)),
                    _ => Err("< requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert(">".to_string(), Value::BuiltinFunction {
            name: ">".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Bool(a > b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Bool(a > b)),
                    _ => Err("> requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("<=".to_string(), Value::BuiltinFunction {
            name: "<=".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Bool(a <= b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Bool(a <= b)),
                    _ => Err("<= requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert(">=".to_string(), Value::BuiltinFunction {
            name: ">=".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(a), Value::Integer32(b)) => Ok(Value::Bool(a >= b)),
                    (Value::Integer64(a), Value::Integer64(b)) => Ok(Value::Bool(a >= b)),
                    _ => Err(">= requires two integers of the same type".to_string()),
                }
            },
        });
        
        env.values.insert("and".to_string(), Value::BuiltinFunction {
            name: "and".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                    _ => Err("and requires two booleans".to_string()),
                }
            },
        });
        
        env.values.insert("or".to_string(), Value::BuiltinFunction {
            name: "or".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                    _ => Err("or requires two booleans".to_string()),
                }
            },
        });
        
        env.values.insert("not".to_string(), Value::BuiltinFunction {
            name: "not".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    _ => Err("not requires a boolean".to_string()),
                }
            },
        });
        
        env.values.insert("print".to_string(), Value::BuiltinFunction {
            name: "print".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        print!("{}", s);
                        Ok(Value::String(s.clone()))
                    }
                    v => {
                        print!("{}", v);
                        Ok(v.clone())
                    }
                }
            },
        });
        
        env.values.insert("println".to_string(), Value::BuiltinFunction {
            name: "println".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::String(s) => {
                        println!("{}", s);
                        Ok(Value::String(s.clone()))
                    }
                    v => {
                        println!("{}", v);
                        Ok(v.clone())
                    }
                }
            },
        });
        
        env.values.insert("type-of".to_string(), Value::BuiltinFunction {
            name: "type-of".to_string(),
            arity: 1,
            func: |args| {
                Ok(Value::String(args[0].type_name().to_string()))
            },
        });
        
        // List operations
        env.values.insert("cons".to_string(), Value::BuiltinFunction {
            name: "cons".to_string(),
            arity: 2,
            func: |args| {
                match &args[1] {
                    Value::List(lst) => {
                        let mut new_list = vec![args[0].clone()];
                        new_list.extend(lst.clone());
                        Ok(Value::List(new_list))
                    }
                    Value::Nil => {
                        Ok(Value::List(vec![args[0].clone()]))
                    }
                    _ => Err("cons requires a list as second argument".to_string()),
                }
            },
        });
        
        env.values.insert("car".to_string(), Value::BuiltinFunction {
            name: "car".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(lst) if !lst.is_empty() => Ok(lst[0].clone()),
                    Value::List(_) | Value::Nil => Err("car of empty list".to_string()),
                    _ => Err("car requires a list".to_string()),
                }
            },
        });
        
        env.values.insert("cdr".to_string(), Value::BuiltinFunction {
            name: "cdr".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(lst) if !lst.is_empty() => {
                        if lst.len() == 1 {
                            Ok(Value::Nil)
                        } else {
                            Ok(Value::List(lst[1..].to_vec()))
                        }
                    }
                    Value::List(_) | Value::Nil => Err("cdr of empty list".to_string()),
                    _ => Err("cdr requires a list".to_string()),
                }
            },
        });
        
        env.values.insert("null?".to_string(), Value::BuiltinFunction {
            name: "null?".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::Nil => Ok(Value::Bool(true)),
                    Value::List(lst) => Ok(Value::Bool(lst.is_empty())),
                    _ => Ok(Value::Bool(false)),
                }
            },
        });
        
        env.values.insert("length".to_string(), Value::BuiltinFunction {
            name: "length".to_string(),
            arity: 1,
            func: |args| {
                match &args[0] {
                    Value::List(lst) => Ok(Value::Integer32(lst.len() as i32)),
                    Value::Nil => Ok(Value::Integer32(0)),
                    _ => Err("length requires a list".to_string()),
                }
            },
        });
        
        env.values.insert("append".to_string(), Value::BuiltinFunction {
            name: "append".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::List(lst1), Value::List(lst2)) => {
                        let mut new_list = lst1.clone();
                        new_list.extend(lst2.clone());
                        Ok(Value::List(new_list))
                    }
                    (Value::Nil, Value::List(lst)) => Ok(Value::List(lst.clone())),
                    (Value::List(lst), Value::Nil) => Ok(Value::List(lst.clone())),
                    (Value::Nil, Value::Nil) => Ok(Value::Nil),
                    _ => Err("append requires two lists".to_string()),
                }
            },
        });
        
        env.values.insert("nth".to_string(), Value::BuiltinFunction {
            name: "nth".to_string(),
            arity: 2,
            func: |args| {
                match (&args[0], &args[1]) {
                    (Value::Integer32(n), Value::List(lst)) => {
                        if *n < 0 || *n as usize >= lst.len() {
                            Err(format!("Index {} out of bounds", n))
                        } else {
                            Ok(lst[*n as usize].clone())
                        }
                    }
                    (Value::Integer32(_), Value::Nil) => Err("Index out of bounds".to_string()),
                    _ => Err("nth requires an integer index and a list".to_string()),
                }
            },
        });
        
        env
    }
    
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.values.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.get(name))
        })
    }
    
    pub fn set(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }
    
    pub fn extend(&self) -> Self {
        Environment {
            values: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }
}
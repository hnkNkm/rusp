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
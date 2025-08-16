use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer32(i32),
    Integer64(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Symbol(String),
    List(Vec<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Let {
        name: String,
        type_ann: Option<Type>,
        value: Box<Expr>,
        body: Option<Box<Expr>>,  // Optional body for let-in expressions
    },
    Defn {
        name: String,
        params: Vec<(String, Type)>,
        return_type: Type,
        body: Box<Expr>,
    },
    Lambda {
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
        body: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    F64,
    Bool,
    String,
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Inferred,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "String"),
            Type::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Inferred => write!(f, "_"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Integer32(n) => write!(f, "{}", n),
            Expr::Integer64(n) => write!(f, "{}", n),
            Expr::Float(n) => write!(f, "{}", n),
            Expr::Bool(b) => write!(f, "{}", b),
            Expr::String(s) => write!(f, "\"{}\"", s),
            Expr::Symbol(s) => write!(f, "{}", s),
            Expr::List(exprs) => {
                write!(f, "(")?;
                for (i, expr) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
            Expr::If { condition, then_branch, else_branch } => {
                write!(f, "(if {} {} {})", condition, then_branch, else_branch)
            }
            Expr::Let { name, type_ann, value, body } => {
                if let Some(b) = body {
                    if let Some(ty) = type_ann {
                        write!(f, "(let {} {} {} {})", name, ty, value, b)
                    } else {
                        write!(f, "(let {} {} {})", name, value, b)
                    }
                } else {
                    if let Some(ty) = type_ann {
                        write!(f, "(let {} {} {})", name, ty, value)
                    } else {
                        write!(f, "(let {} {})", name, value)
                    }
                }
            }
            Expr::Defn { name, params, return_type, body } => {
                write!(f, "(defn {} [", name)?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}: {}", param_name, param_type)?;
                }
                write!(f, "] -> {} {})", return_type, body)
            }
            Expr::Lambda { params, return_type, body } => {
                write!(f, "(fn [")?;
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}: {}", param_name, param_type)?;
                }
                write!(f, "]")?;
                if let Some(rt) = return_type {
                    write!(f, " -> {}", rt)?;
                }
                write!(f, " {})", body)
            }
            Expr::Call { func, args } => {
                write!(f, "({}", func)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}
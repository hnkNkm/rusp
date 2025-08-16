#[cfg(test)]
mod tests {
    use crate::ast::{Expr, Type};
    use crate::parser;
    
    fn parse(input: &str) -> Result<Expr, String> {
        parser::parse(input).map_err(|e| e.to_string())
    }
    
    #[test]
    fn test_parse_integers() {
        // i32
        let result = parse("42").unwrap();
        assert_eq!(result, Expr::Integer32(42));
        
        // negative i32
        let result = parse("-42").unwrap();
        assert_eq!(result, Expr::Integer32(-42));
        
        // i64 (large number)
        let result = parse("9223372036854775807").unwrap();
        assert_eq!(result, Expr::Integer64(9223372036854775807));
    }
    
    #[test]
    fn test_parse_floats() {
        let result = parse("3.14").unwrap();
        assert_eq!(result, Expr::Float(3.14));
        
        let result = parse("-2.5").unwrap();
        assert_eq!(result, Expr::Float(-2.5));
    }
    
    #[test]
    fn test_parse_booleans() {
        let result = parse("true").unwrap();
        assert_eq!(result, Expr::Bool(true));
        
        let result = parse("false").unwrap();
        assert_eq!(result, Expr::Bool(false));
    }
    
    #[test]
    fn test_parse_strings() {
        let result = parse("\"hello\"").unwrap();
        assert_eq!(result, Expr::String("hello".to_string()));
        
        let result = parse("\"\"").unwrap();
        assert_eq!(result, Expr::String("".to_string()));
    }
    
    #[test]
    fn test_parse_symbols() {
        let result = parse("foo").unwrap();
        assert_eq!(result, Expr::Symbol("foo".to_string()));
        
        let result = parse("foo-bar").unwrap();
        assert_eq!(result, Expr::Symbol("foo-bar".to_string()));
    }
    
    #[test]
    fn test_parse_if_expression() {
        let result = parse("(if true 1 2)").unwrap();
        match result {
            Expr::If { condition, then_branch, else_branch } => {
                assert_eq!(*condition, Expr::Bool(true));
                assert_eq!(*then_branch, Expr::Integer32(1));
                assert_eq!(*else_branch, Expr::Integer32(2));
            }
            _ => panic!("Expected If expression"),
        }
    }
    
    #[test]
    fn test_parse_let_simple() {
        // Without type annotation
        let result = parse("(let x 42)").unwrap();
        match result {
            Expr::Let { name, type_ann, value, body } => {
                assert_eq!(name, "x");
                assert_eq!(type_ann, None);
                assert_eq!(*value, Expr::Integer32(42));
                assert_eq!(body, None);
            }
            _ => panic!("Expected Let expression"),
        }
        
        // With type annotation (colon syntax)
        let result = parse("(let x: i32 42)").unwrap();
        match result {
            Expr::Let { name, type_ann, value, body } => {
                assert_eq!(name, "x");
                assert_eq!(type_ann, Some(Type::I32));
                assert_eq!(*value, Expr::Integer32(42));
                assert_eq!(body, None);
            }
            _ => panic!("Expected Let expression"),
        }
    }
    
    #[test]
    fn test_parse_let_in() {
        // let-in expression
        let result = parse("(let x: i32 10 (+ x 5))").unwrap();
        match result {
            Expr::Let { name, type_ann, value, body } => {
                assert_eq!(name, "x");
                assert_eq!(type_ann, Some(Type::I32));
                assert_eq!(*value, Expr::Integer32(10));
                assert!(body.is_some());
                
                if let Some(body_expr) = body {
                    match &*body_expr {
                        Expr::List(list) => {
                            assert_eq!(list[0], Expr::Symbol("+".to_string()));
                            assert_eq!(list[1], Expr::Symbol("x".to_string()));
                            assert_eq!(list[2], Expr::Integer32(5));
                        }
                        _ => panic!("Expected List in body"),
                    }
                }
            }
            _ => panic!("Expected Let expression"),
        }
    }
    
    #[test]
    fn test_parse_defn() {
        let result = parse("(defn add [a: i32 b: i32] -> i32 (+ a b))").unwrap();
        match result {
            Expr::Defn { name, params, return_type, body } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], ("a".to_string(), Type::I32));
                assert_eq!(params[1], ("b".to_string(), Type::I32));
                assert_eq!(return_type, Type::I32);
                
                match &*body {
                    Expr::List(list) => {
                        assert_eq!(list[0], Expr::Symbol("+".to_string()));
                        assert_eq!(list[1], Expr::Symbol("a".to_string()));
                        assert_eq!(list[2], Expr::Symbol("b".to_string()));
                    }
                    _ => panic!("Expected List in body"),
                }
            }
            _ => panic!("Expected Defn expression"),
        }
    }
    
    #[test]
    fn test_parse_lambda() {
        let result = parse("(fn [x: i32] -> i32 (* x 2))").unwrap();
        match result {
            Expr::Lambda { params, return_type, body } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], ("x".to_string(), Type::I32));
                assert_eq!(return_type, Some(Type::I32));
                
                match &*body {
                    Expr::List(list) => {
                        assert_eq!(list[0], Expr::Symbol("*".to_string()));
                        assert_eq!(list[1], Expr::Symbol("x".to_string()));
                        assert_eq!(list[2], Expr::Integer32(2));
                    }
                    _ => panic!("Expected List in body"),
                }
            }
            _ => panic!("Expected Lambda expression"),
        }
    }
    
    #[test]
    fn test_parse_function_call() {
        let result = parse("(add 1 2)").unwrap();
        match result {
            Expr::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list[0], Expr::Symbol("add".to_string()));
                assert_eq!(list[1], Expr::Integer32(1));
                assert_eq!(list[2], Expr::Integer32(2));
            }
            _ => panic!("Expected List (function call)"),
        }
    }
    
    #[test]
    fn test_parse_nested_expressions() {
        let result = parse("(+ (* 2 3) (- 10 5))").unwrap();
        match result {
            Expr::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list[0], Expr::Symbol("+".to_string()));
                
                // First nested expression (* 2 3)
                match &list[1] {
                    Expr::List(inner) => {
                        assert_eq!(inner[0], Expr::Symbol("*".to_string()));
                        assert_eq!(inner[1], Expr::Integer32(2));
                        assert_eq!(inner[2], Expr::Integer32(3));
                    }
                    _ => panic!("Expected nested List"),
                }
                
                // Second nested expression (- 10 5)
                match &list[2] {
                    Expr::List(inner) => {
                        assert_eq!(inner[0], Expr::Symbol("-".to_string()));
                        assert_eq!(inner[1], Expr::Integer32(10));
                        assert_eq!(inner[2], Expr::Integer32(5));
                    }
                    _ => panic!("Expected nested List"),
                }
            }
            _ => panic!("Expected List"),
        }
    }
    
    #[test]
    fn test_parse_empty_list() {
        let result = parse("()").unwrap();
        // Empty list parses as an empty list, not an error
        assert_eq!(result, Expr::List(vec![]));
    }
    
    #[test]
    fn test_parse_i64_type() {
        let result = parse("(let x: i64 9223372036854775807)").unwrap();
        match result {
            Expr::Let { name, type_ann, value, body } => {
                assert_eq!(name, "x");
                assert_eq!(type_ann, Some(Type::I64));
                assert_eq!(*value, Expr::Integer64(9223372036854775807));
                assert_eq!(body, None);
            }
            _ => panic!("Expected Let expression"),
        }
    }
}
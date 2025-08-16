#[cfg(test)]
mod tests {
    use crate::ast::Type;
    use crate::env::{Environment, Value};
    use crate::eval::eval;
    use crate::parser;
    use crate::types::{type_check, TypeEnv};
    
    fn eval_str(input: &str) -> Result<Value, String> {
        let expr = parser::parse(input).map_err(|e| e.to_string())?;
        let mut env = Environment::new();
        eval(&expr, &mut env)
    }
    
    fn type_check_str(input: &str) -> Result<Type, String> {
        let expr = parser::parse(input).map_err(|e| e.to_string())?;
        let mut env = TypeEnv::new();
        type_check(&expr, &mut env)
    }
    
    #[test]
    fn test_eval_integers() {
        let result = eval_str("42").unwrap();
        assert!(matches!(result, Value::Integer32(42)));
        
        let result = eval_str("-10").unwrap();
        assert!(matches!(result, Value::Integer32(-10)));
        
        let result = eval_str("9223372036854775807").unwrap();
        assert!(matches!(result, Value::Integer64(9223372036854775807)));
    }
    
    #[test]
    fn test_eval_arithmetic() {
        // Addition
        let result = eval_str("(+ 2 3)").unwrap();
        assert!(matches!(result, Value::Integer32(5)));
        
        // Subtraction
        let result = eval_str("(- 10 4)").unwrap();
        assert!(matches!(result, Value::Integer32(6)));
        
        // Multiplication
        let result = eval_str("(* 3 4)").unwrap();
        assert!(matches!(result, Value::Integer32(12)));
        
        // Division
        let result = eval_str("(/ 10 2)").unwrap();
        assert!(matches!(result, Value::Integer32(5)));
        
        // Nested arithmetic
        let result = eval_str("(+ (* 2 3) (- 10 5))").unwrap();
        assert!(matches!(result, Value::Integer32(11)));
    }
    
    #[test]
    fn test_eval_i64_arithmetic() {
        // Test i64 addition (both numbers must be i64)
        let result = eval_str("(+ 4611686018427387904 4611686018427387903)").unwrap();
        assert!(matches!(result, Value::Integer64(9223372036854775807)));
        
        // Test i64 subtraction
        let result = eval_str("(- 9223372036854775807 9223372036854775806)").unwrap();
        assert!(matches!(result, Value::Integer64(1)));
        
        // Test i64 division
        let result = eval_str("(/ 9223372036854775806 4611686018427387903)").unwrap();
        assert!(matches!(result, Value::Integer64(2)));
    }
    
    #[test]
    fn test_eval_float_arithmetic() {
        let result = eval_str("(+. 2.5 3.5)").unwrap();
        match result {
            Value::Float(f) => assert!((f - 6.0).abs() < 0.001),
            _ => panic!("Expected Float"),
        }
        
        let result = eval_str("(-. 10.0 3.5)").unwrap();
        match result {
            Value::Float(f) => assert!((f - 6.5).abs() < 0.001),
            _ => panic!("Expected Float"),
        }
    }
    
    #[test]
    fn test_eval_comparisons() {
        let result = eval_str("(= 5 5)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        let result = eval_str("(= 5 3)").unwrap();
        assert!(matches!(result, Value::Bool(false)));
        
        let result = eval_str("(< 3 5)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        let result = eval_str("(> 5 3)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        let result = eval_str("(<= 5 5)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        let result = eval_str("(>= 5 3)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }
    
    #[test]
    fn test_eval_logical() {
        let result = eval_str("(and true true)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        let result = eval_str("(and true false)").unwrap();
        assert!(matches!(result, Value::Bool(false)));
        
        let result = eval_str("(or false true)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        let result = eval_str("(or false false)").unwrap();
        assert!(matches!(result, Value::Bool(false)));
        
        let result = eval_str("(not true)").unwrap();
        assert!(matches!(result, Value::Bool(false)));
        
        let result = eval_str("(not false)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }
    
    #[test]
    fn test_eval_if() {
        let result = eval_str("(if true 10 20)").unwrap();
        assert!(matches!(result, Value::Integer32(10)));
        
        let result = eval_str("(if false 10 20)").unwrap();
        assert!(matches!(result, Value::Integer32(20)));
        
        let result = eval_str("(if (< 3 5) 100 200)").unwrap();
        assert!(matches!(result, Value::Integer32(100)));
    }
    
    #[test]
    fn test_eval_let() {
        // Simple let
        let result = eval_str("(let x 10)").unwrap();
        assert!(matches!(result, Value::Integer32(10)));
        
        // Let with type annotation
        let result = eval_str("(let x: i32 42)").unwrap();
        assert!(matches!(result, Value::Integer32(42)));
        
        // Let with i64
        let result = eval_str("(let x: i64 9223372036854775807)").unwrap();
        assert!(matches!(result, Value::Integer64(9223372036854775807)));
    }
    
    #[test]
    fn test_eval_let_in() {
        // Let-in expression
        let result = eval_str("(let x: i32 10 (+ x 5))").unwrap();
        assert!(matches!(result, Value::Integer32(15)));
        
        // Nested let-in
        let result = eval_str("(let x: i32 10 (let y: i32 20 (+ x y)))").unwrap();
        assert!(matches!(result, Value::Integer32(30)));
        
        // Let-in with scope
        let result = eval_str("(let x: i32 5 (let x: i32 10 x))").unwrap();
        assert!(matches!(result, Value::Integer32(10))); // Inner x shadows outer x
    }
    
    #[test]
    fn test_eval_function_definition() {
        let mut env = Environment::new();
        
        // Define a function
        let expr = parser::parse("(defn add [a: i32 b: i32] -> i32 (+ a b))").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Function { .. }));
        
        // Call the function
        let expr = parser::parse("(add 5 3)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(8)));
    }
    
    #[test]
    fn test_eval_recursive_factorial() {
        let mut env = Environment::new();
        
        // Define factorial
        let expr = parser::parse(
            "(defn factorial [n: i32] -> i32 (if (= n 0) 1 (* n (factorial (- n 1)))))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test factorial(5) = 120
        let expr = parser::parse("(factorial 5)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(120)));
        
        // Test factorial(0) = 1
        let expr = parser::parse("(factorial 0)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(1)));
    }
    
    #[test]
    fn test_eval_recursive_fibonacci() {
        let mut env = Environment::new();
        
        // Define fibonacci
        let expr = parser::parse(
            "(defn fib [n: i32] -> i32 (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test fib(10) = 55
        let expr = parser::parse("(fib 10)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(55)));
        
        // Test fib(0) = 0, fib(1) = 1
        let expr = parser::parse("(fib 0)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(0)));
        
        let expr = parser::parse("(fib 1)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(1)));
    }
    
    #[test]
    fn test_eval_lambda() {
        let mut env = Environment::new();
        
        // Define a variable holding a lambda
        let expr = parser::parse("(let double (fn [x: i32] -> i32 (* x 2)))").unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Call the lambda
        let expr = parser::parse("(double 5)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(10)));
    }
    
    #[test]
    fn test_eval_function_with_let() {
        let mut env = Environment::new();
        
        // Function using let-in
        let expr = parser::parse(
            "(defn test-scope [x: i32] -> i32 (let y: i32 (* x 2) (+ x y)))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test test-scope(10) = 30
        let expr = parser::parse("(test-scope 10)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(30)));
    }
    
    #[test]
    fn test_type_check_arithmetic() {
        let ty = type_check_str("(+ 1 2)").unwrap();
        assert_eq!(ty, Type::I32);
        
        let ty = type_check_str("(+. 1.0 2.0)").unwrap();
        assert_eq!(ty, Type::F64);
    }
    
    #[test]
    fn test_type_check_comparison() {
        let ty = type_check_str("(< 1 2)").unwrap();
        assert_eq!(ty, Type::Bool);
        
        let ty = type_check_str("(= 5 5)").unwrap();
        assert_eq!(ty, Type::Bool);
    }
    
    #[test]
    fn test_type_check_if() {
        let ty = type_check_str("(if true 10 20)").unwrap();
        assert_eq!(ty, Type::I32);
        
        let ty = type_check_str("(if (< 1 2) 3.14 2.71)").unwrap();
        assert_eq!(ty, Type::F64);
    }
    
    #[test]
    fn test_type_check_let() {
        let ty = type_check_str("(let x: i32 42)").unwrap();
        assert_eq!(ty, Type::I32);
        
        let ty = type_check_str("(let x: i64 9223372036854775807)").unwrap();
        assert_eq!(ty, Type::I64);
    }
    
    #[test]
    fn test_type_check_function() {
        let mut env = TypeEnv::new();
        
        // Type check function definition
        let expr = parser::parse("(defn add [a: i32 b: i32] -> i32 (+ a b))").unwrap();
        let ty = type_check(&expr, &mut env).unwrap();
        assert_eq!(ty, Type::Function {
            params: vec![Type::I32, Type::I32],
            return_type: Box::new(Type::I32),
        });
        
        // Type check function call
        let expr = parser::parse("(add 1 2)").unwrap();
        let ty = type_check(&expr, &mut env).unwrap();
        assert_eq!(ty, Type::I32);
    }
    
    #[test]
    fn test_type_error_mismatch() {
        // Type mismatch in if branches
        let result = type_check_str("(if true 10 3.14)");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same type"));
        
        // Wrong argument type
        let result = type_check_str("(and 1 2)");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_type_error_undefined() {
        let result = type_check_str("undefined_var");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Undefined"));
    }
    
    #[test]
    fn test_eval_print() {
        // print should work with any type
        let result = eval_str("(print \"Hello\")").unwrap();
        assert!(matches!(result, Value::String(_)));
        
        let result = eval_str("(print 42)").unwrap();
        assert!(matches!(result, Value::Integer32(42)));
        
        let result = eval_str("(println true)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }
    
    #[test]
    fn test_eval_division_by_zero() {
        let result = eval_str("(/ 10 0)");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
        
        let result = eval_str("(/. 10.0 0.0)");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }
}
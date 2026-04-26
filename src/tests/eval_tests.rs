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
    
    // List operation tests
    
    #[test]
    fn test_eval_list_creation() {
        // Create a list using the list special form
        let result = eval_str("(list 1 2 3)").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(1)));
                assert!(matches!(values[1], Value::Integer32(2)));
                assert!(matches!(values[2], Value::Integer32(3)));
            }
            _ => panic!("Expected List"),
        }
        
        // Empty list
        let result = eval_str("(list)").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 0);
            }
            _ => panic!("Expected empty List"),
        }
        
        // nil is empty list
        let result = eval_str("nil").unwrap();
        assert!(matches!(result, Value::Nil));
    }
    
    #[test]
    fn test_eval_cons() {
        // Basic cons
        let result = eval_str("(cons 0 (list 1 2 3))").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 4);
                assert!(matches!(values[0], Value::Integer32(0)));
                assert!(matches!(values[1], Value::Integer32(1)));
            }
            _ => panic!("Expected List"),
        }
        
        // Cons with nil
        let result = eval_str("(cons 1 nil)").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 1);
                assert!(matches!(values[0], Value::Integer32(1)));
            }
            _ => panic!("Expected List"),
        }
        
        // Build list with cons
        let result = eval_str("(cons 1 (cons 2 (cons 3 nil)))").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(1)));
                assert!(matches!(values[1], Value::Integer32(2)));
                assert!(matches!(values[2], Value::Integer32(3)));
            }
            _ => panic!("Expected List"),
        }
    }
    
    #[test]
    fn test_eval_car() {
        // Get first element
        let result = eval_str("(car (list 1 2 3))").unwrap();
        assert!(matches!(result, Value::Integer32(1)));
        
        // Car of single element list
        let result = eval_str("(car (list 42))").unwrap();
        assert!(matches!(result, Value::Integer32(42)));
        
        // Car of empty list should error
        let result = eval_str("(car nil)");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("car of empty list"));
    }
    
    #[test]
    fn test_eval_cdr() {
        // Get rest of list
        let result = eval_str("(cdr (list 1 2 3))").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 2);
                assert!(matches!(values[0], Value::Integer32(2)));
                assert!(matches!(values[1], Value::Integer32(3)));
            }
            _ => panic!("Expected List"),
        }
        
        // Cdr of single element list is nil
        let result = eval_str("(cdr (list 1))").unwrap();
        assert!(matches!(result, Value::Nil));
        
        // Cdr of empty list should error
        let result = eval_str("(cdr nil)");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cdr of empty list"));
    }
    
    #[test]
    fn test_eval_null_check() {
        // null? on nil
        let result = eval_str("(null? nil)").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        // null? on empty list
        let result = eval_str("(null? (list))").unwrap();
        assert!(matches!(result, Value::Bool(true)));
        
        // null? on non-empty list
        let result = eval_str("(null? (list 1 2 3))").unwrap();
        assert!(matches!(result, Value::Bool(false)));
        
        // null? on cdr of single element list
        let result = eval_str("(null? (cdr (list 1)))").unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }
    
    #[test]
    fn test_eval_length() {
        // Length of list
        let result = eval_str("(length (list 1 2 3))").unwrap();
        assert!(matches!(result, Value::Integer32(3)));
        
        // Length of empty list
        let result = eval_str("(length (list))").unwrap();
        assert!(matches!(result, Value::Integer32(0)));
        
        // Length of nil
        let result = eval_str("(length nil)").unwrap();
        assert!(matches!(result, Value::Integer32(0)));
        
        // Length of constructed list
        let result = eval_str("(length (cons 1 (cons 2 nil)))").unwrap();
        assert!(matches!(result, Value::Integer32(2)));
    }
    
    #[test]
    fn test_eval_append() {
        // Append two lists
        let result = eval_str("(append (list 1 2) (list 3 4))").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 4);
                assert!(matches!(values[0], Value::Integer32(1)));
                assert!(matches!(values[3], Value::Integer32(4)));
            }
            _ => panic!("Expected List"),
        }
        
        // Append with nil
        let result = eval_str("(append nil (list 1 2))").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 2);
            }
            _ => panic!("Expected List"),
        }
        
        // Append to nil
        let result = eval_str("(append (list 1 2) nil)").unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 2);
            }
            _ => panic!("Expected List"),
        }
        
        // Append nil to nil
        let result = eval_str("(append nil nil)").unwrap();
        assert!(matches!(result, Value::Nil));
    }
    
    #[test]
    fn test_eval_nth() {
        // Access by index
        let result = eval_str("(nth 0 (list 10 20 30))").unwrap();
        assert!(matches!(result, Value::Integer32(10)));
        
        let result = eval_str("(nth 1 (list 10 20 30))").unwrap();
        assert!(matches!(result, Value::Integer32(20)));
        
        let result = eval_str("(nth 2 (list 10 20 30))").unwrap();
        assert!(matches!(result, Value::Integer32(30)));
        
        // Out of bounds
        let result = eval_str("(nth 3 (list 10 20 30))");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
        
        // Negative index
        let result = eval_str("(nth -1 (list 10 20 30))");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
        
        // nth on nil
        let result = eval_str("(nth 0 nil)");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }
    
    #[test]
    fn test_eval_recursive_sum_list() {
        let mut env = Environment::new();
        
        // Define sum-list
        let expr = parser::parse(
            "(defn sum-list [lst: List<i32>] -> i32 (if (null? lst) 0 (+ (car lst) (sum-list (cdr lst)))))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test sum-list
        let expr = parser::parse("(sum-list (list 1 2 3 4 5))").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(15)));
        
        // Empty list
        let expr = parser::parse("(sum-list nil)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(0)));
    }
    
    #[test]
    fn test_eval_recursive_map_inc() {
        let mut env = Environment::new();
        
        // Define map-inc
        let expr = parser::parse(
            "(defn map-inc [lst: List<i32>] -> List<i32> (if (null? lst) nil (cons (+ (car lst) 1) (map-inc (cdr lst)))))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test map-inc
        let expr = parser::parse("(map-inc (list 1 2 3))").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(2)));
                assert!(matches!(values[1], Value::Integer32(3)));
                assert!(matches!(values[2], Value::Integer32(4)));
            }
            _ => panic!("Expected List"),
        }
        
        // Empty list
        let expr = parser::parse("(map-inc nil)").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        assert!(matches!(result, Value::Nil));
    }
    
    #[test]
    fn test_eval_recursive_filter_even() {
        let mut env = Environment::new();
        
        // Define is-even helper
        let expr = parser::parse(
            "(defn is-even [n: i32] -> bool (= (* (/ n 2) 2) n))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Define filter-even
        let expr = parser::parse(
            "(defn filter-even [lst: List<i32>] -> List<i32> (if (null? lst) nil (if (is-even (car lst)) (cons (car lst) (filter-even (cdr lst))) (filter-even (cdr lst)))))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test filter-even
        let expr = parser::parse("(filter-even (list 1 2 3 4 5 6))").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(2)));
                assert!(matches!(values[1], Value::Integer32(4)));
                assert!(matches!(values[2], Value::Integer32(6)));
            }
            _ => panic!("Expected List"),
        }
    }
    
    #[test]
    fn test_eval_recursive_reverse() {
        let mut env = Environment::new();
        
        // Define reverse
        let expr = parser::parse(
            "(defn reverse [lst: List<i32>] -> List<i32> (if (null? lst) nil (append (reverse (cdr lst)) (list (car lst)))))"
        ).unwrap();
        eval(&expr, &mut env).unwrap();
        
        // Test reverse
        let expr = parser::parse("(reverse (list 1 2 3 4))").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        match result {
            Value::List(ref values) => {
                assert_eq!(values.len(), 4);
                assert!(matches!(values[0], Value::Integer32(4)));
                assert!(matches!(values[1], Value::Integer32(3)));
                assert!(matches!(values[2], Value::Integer32(2)));
                assert!(matches!(values[3], Value::Integer32(1)));
            }
            _ => panic!("Expected List"),
        }
    }
    
    #[test]
    fn test_type_check_list() {
        // Type check list creation
        let ty = type_check_str("(list 1 2 3)").unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::I32)));
        
        // Type check cons
        let ty = type_check_str("(cons 1 (list 2 3))").unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::I32)));
        
        // Type check car
        let ty = type_check_str("(car (list 1 2 3))").unwrap();
        assert_eq!(ty, Type::I32);
        
        // Type check cdr
        let ty = type_check_str("(cdr (list 1 2 3))").unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::I32)));
        
        // Type check null?
        let ty = type_check_str("(null? (list 1 2 3))").unwrap();
        assert_eq!(ty, Type::Bool);
        
        // Type check length
        let ty = type_check_str("(length (list 1 2 3))").unwrap();
        assert_eq!(ty, Type::I32);
        
        // Type check append
        let ty = type_check_str("(append (list 1 2) (list 3 4))").unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::I32)));
        
        // Type check nth
        let ty = type_check_str("(nth 0 (list 1 2 3))").unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_type_check_list_heterogeneous_rejected() {
        // Mixing i32 and String in (list ...) must be a type error.
        let result = type_check_str("(list 1 \"hello\" 3)");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("List element type mismatch"),
            "unexpected error message: {}",
            msg
        );
    }

    #[test]
    fn test_type_check_list_homogeneous_ok() {
        // Sanity: homogeneous list still type-checks.
        let ty = type_check_str("(list \"a\" \"b\" \"c\")").unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::String)));
    }

    #[test]
    fn test_type_check_list_functions() {
        let mut env = TypeEnv::new();
        
        // Type check sum-list function
        let expr = parser::parse(
            "(defn sum-list [lst: List<i32>] -> i32 (if (null? lst) 0 (+ (car lst) (sum-list (cdr lst)))))"
        ).unwrap();
        let ty = type_check(&expr, &mut env).unwrap();
        assert_eq!(ty, Type::Function {
            params: vec![Type::List(Box::new(Type::I32))],
            return_type: Box::new(Type::I32),
        });
        
        // Type check map-inc function
        let expr = parser::parse(
            "(defn map-inc [lst: List<i32>] -> List<i32> (if (null? lst) nil (cons (+ (car lst) 1) (map-inc (cdr lst)))))"
        ).unwrap();
        let ty = type_check(&expr, &mut env).unwrap();
        assert_eq!(ty, Type::Function {
            params: vec![Type::List(Box::new(Type::I32))],
            return_type: Box::new(Type::List(Box::new(Type::I32))),
        });
    }

    // Higher-order function tests: map / filter / fold
    //
    // These exercise both the evaluator (function values are invoked inside
    // the special form) and the type checker (which threads the element type
    // through the function signature).

    #[test]
    fn test_eval_map_with_lambda() {
        let mut env = Environment::new();
        let expr = parser::parse("(map (fn [x: i32] -> i32 (* x x)) (list 1 2 3))").unwrap();
        let result = eval(&expr, &mut env).unwrap();
        match result {
            Value::List(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(1)));
                assert!(matches!(values[1], Value::Integer32(4)));
                assert!(matches!(values[2], Value::Integer32(9)));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_map_with_defn() {
        // map over a named function; also exercises the function-name lookup
        // path in apply_function.
        let mut env = Environment::new();
        eval(
            &parser::parse("(defn inc [x: i32] -> i32 (+ x 1))").unwrap(),
            &mut env,
        )
        .unwrap();
        let result = eval(
            &parser::parse("(map inc (list 10 20 30))").unwrap(),
            &mut env,
        )
        .unwrap();
        match result {
            Value::List(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(11)));
                assert!(matches!(values[1], Value::Integer32(21)));
                assert!(matches!(values[2], Value::Integer32(31)));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_filter() {
        let mut env = Environment::new();
        let expr = parser::parse(
            "(filter (fn [x: i32] -> bool (> x 2)) (list 1 2 3 4 5))",
        )
        .unwrap();
        let result = eval(&expr, &mut env).unwrap();
        match result {
            Value::List(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], Value::Integer32(3)));
                assert!(matches!(values[1], Value::Integer32(4)));
                assert!(matches!(values[2], Value::Integer32(5)));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_filter_empty_result_is_nil() {
        let result = eval_str(
            "(filter (fn [x: i32] -> bool (> x 100)) (list 1 2 3))",
        )
        .unwrap();
        assert!(matches!(result, Value::Nil));
    }

    #[test]
    fn test_eval_fold_sum() {
        let result = eval_str(
            "(fold (fn [acc: i32 x: i32] -> i32 (+ acc x)) 0 (list 1 2 3 4 5))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(15)));
    }

    #[test]
    fn test_eval_fold_empty_list_returns_init() {
        let result = eval_str(
            "(fold (fn [acc: i32 x: i32] -> i32 (+ acc x)) 42 nil)",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(42)));
    }

    #[test]
    fn test_type_check_map() {
        let ty = type_check_str(
            "(map (fn [x: i32] -> i32 (* x x)) (list 1 2 3))",
        )
        .unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::I32)));
    }

    #[test]
    fn test_type_check_map_changes_element_type() {
        // i32 -> bool, so result is List<bool>
        let ty = type_check_str(
            "(map (fn [x: i32] -> bool (> x 0)) (list 1 -2 3))",
        )
        .unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::Bool)));
    }

    #[test]
    fn test_type_check_filter() {
        let ty = type_check_str(
            "(filter (fn [x: i32] -> bool (> x 0)) (list 1 -2 3))",
        )
        .unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::I32)));
    }

    #[test]
    fn test_type_check_fold() {
        let ty = type_check_str(
            "(fold (fn [acc: i32 x: i32] -> i32 (+ acc x)) 0 (list 1 2 3))",
        )
        .unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_type_check_map_wrong_function_arity_rejected() {
        let result = type_check_str(
            "(map (fn [x: i32 y: i32] -> i32 (+ x y)) (list 1 2 3))",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unary function"));
    }

    #[test]
    fn test_type_check_filter_non_bool_predicate_rejected() {
        let result = type_check_str(
            "(filter (fn [x: i32] -> i32 x) (list 1 2 3))",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("bool"));
    }

    #[test]
    fn test_type_check_fold_mismatched_acc_rejected() {
        // init is i32 but function returns bool — incompatible accumulator.
        let result = type_check_str(
            "(fold (fn [acc: i32 x: i32] -> bool (> acc x)) 0 (list 1 2 3))",
        );
        assert!(result.is_err());
    }

    // ======================================================================
    // Pattern matching
    // ======================================================================

    #[test]
    fn test_eval_match_literal() {
        let result = eval_str("(match 1 (1 \"one\") (2 \"two\") (_ \"other\"))").unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "one"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_eval_match_wildcard_fallthrough() {
        let result = eval_str("(match 99 (1 \"one\") (_ \"other\"))").unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "other"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_eval_match_variable_binding() {
        // Variable patterns bind the scrutinee; first arm always matches.
        let result = eval_str("(match 42 (x (+ x 1)))").unwrap();
        assert!(matches!(result, Value::Integer32(43)));
    }

    #[test]
    fn test_eval_match_nil_on_empty_list() {
        let result = eval_str("(match nil (nil \"empty\") (_ \"nonempty\"))").unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "empty"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_eval_match_nil_on_list_literal() {
        // (list) produces an empty list; nil pattern should match it.
        let result =
            eval_str("(match (list) (nil \"empty\") (_ \"nonempty\"))").unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "empty"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_eval_match_cons_pattern() {
        // Decompose list into head and tail; bind both.
        let result =
            eval_str("(match (list 1 2 3) ((cons h t) h) (nil 0))").unwrap();
        assert!(matches!(result, Value::Integer32(1)));
    }

    #[test]
    fn test_eval_match_cons_nested() {
        // Pattern (cons 1 _) only matches lists starting with 1.
        let result = eval_str(
            "(match (list 1 2 3) ((cons 1 _) \"starts-with-one\") (_ \"other\"))",
        )
        .unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "starts-with-one"),
            _ => panic!("Expected String"),
        }

        let result = eval_str(
            "(match (list 2 3) ((cons 1 _) \"starts-with-one\") (_ \"other\"))",
        )
        .unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "other"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_eval_match_no_arm_matches_error() {
        // No wildcard arm, value doesn't match any literal.
        let result = eval_str("(match 5 (1 \"one\") (2 \"two\"))");
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_match_recursive_sum() {
        // Sum a list via recursion + match.
        let program = "(defn sum [xs: _] -> i32 \
             (match xs \
               (nil 0) \
               ((cons h t) (+ h (sum t)))))";
        let expr = parser::parse(program).map_err(|e| e.to_string()).unwrap();
        let mut env = Environment::new();
        eval(&expr, &mut env).unwrap();

        let call = parser::parse("(sum (list 1 2 3 4 5))")
            .map_err(|e| e.to_string())
            .unwrap();
        let result = eval(&call, &mut env).unwrap();
        assert!(matches!(result, Value::Integer32(15)));
    }

    #[test]
    fn test_type_check_match_literal() {
        let ty = type_check_str("(match 1 (1 \"one\") (_ \"other\"))").unwrap();
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_type_check_match_variable_binding() {
        // `x` is bound to the scrutinee's type (i32), so (+ x 1) type-checks.
        let ty = type_check_str("(match 42 (x (+ x 1)))").unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_type_check_match_cons_binding() {
        // In (cons h t), h has the list's element type, t has the list type.
        let ty = type_check_str("(match (list 1 2 3) ((cons h _) h) (nil 0))").unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_type_check_match_arms_must_agree() {
        // Arms return String and i32 — should error.
        let result = type_check_str("(match 1 (1 \"one\") (_ 2))");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same type"));
    }

    #[test]
    fn test_type_check_match_literal_type_mismatch() {
        // Scrutinee is i32 but pattern is a bool literal.
        let result = type_check_str("(match 1 (true 0) (_ 1))");
        assert!(result.is_err());
    }

    #[test]
    fn test_type_check_match_cons_requires_list() {
        // Scrutinee is an i32; cons pattern is a list-only pattern.
        let result = type_check_str("(match 1 ((cons h t) h) (_ 0))");
        assert!(result.is_err());
    }

    // ======================================================================
    // (list ...) pattern — desugars to cons-chain + nil
    // ======================================================================

    #[test]
    fn test_eval_match_list_pattern_empty() {
        let result = eval_str("(match nil ((list) \"empty\") (_ \"other\"))").unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "empty"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_eval_match_list_pattern_fixed_length() {
        // (list a b c) matches exactly-3-element lists and binds positions.
        let result = eval_str(
            "(match (list 10 20 30) ((list a b c) (+ a (+ b c))) (_ -1))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(60)));
    }

    #[test]
    fn test_eval_match_list_pattern_length_mismatch_falls_through() {
        // (list a b c) should NOT match a 2-element list.
        let result = eval_str(
            "(match (list 1 2) ((list a b c) a) (_ 99))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(99)));
    }

    #[test]
    fn test_eval_match_list_pattern_with_literals() {
        // Positional literal matching.
        let result = eval_str(
            "(match (list 1 2 3) ((list 1 _ _) \"starts-with-one\") (_ \"other\"))",
        )
        .unwrap();
        match result {
            Value::String(s) => assert_eq!(s, "starts-with-one"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_type_check_match_list_pattern_binds_elements() {
        // (list a b) binds both to the element type (i32 here).
        let ty = type_check_str(
            "(match (list 1 2) ((list a b) (+ a b)) (_ 0))",
        )
        .unwrap();
        assert_eq!(ty, Type::I32);
    }

    // ======================================================================
    // (as <pattern> <name>) — alias binding
    // ======================================================================

    #[test]
    fn test_eval_match_as_pattern_binds_whole() {
        // `xs` gets bound to the entire matched list, not just the head.
        let result = eval_str(
            "(match (list 1 2 3) \
               ((as (cons _ _) xs) (length xs)) \
               (_ 0))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(3)));
    }

    #[test]
    fn test_eval_match_as_pattern_on_literal() {
        // `n` bound to the matched value even though the inner pattern is a literal.
        let result = eval_str(
            "(match 42 ((as 42 n) (+ n 1)) (_ 0))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(43)));
    }

    #[test]
    fn test_eval_match_as_pattern_no_bind_on_failure() {
        // When the inner pattern fails, the alias must not leak a binding
        // into the arm. We verify by letting a later arm match and ensuring
        // the arm we enter has no stale `n` visible (it's a fresh scope
        // anyway, but this exercises the code path).
        let result = eval_str(
            "(match 99 ((as 42 n) n) (_ 0))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(0)));
    }

    #[test]
    fn test_type_check_match_as_pattern() {
        // Alias type is the scrutinee type (i32), usable in the body.
        let ty = type_check_str(
            "(match 7 ((as _ n) (+ n 1)))",
        )
        .unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_type_check_match_as_pattern_wraps_cons() {
        // Inner cons pattern binds head; alias gets the full list type.
        let ty = type_check_str(
            "(match (list 1 2 3) \
               ((as (cons h _) xs) (+ h (length xs))) \
               (_ 0))",
        )
        .unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_eval_match_guard_passes() {
        // Guard expression is true → arm is taken.
        let result = eval_str(
            "(match 5 ((guard x (> x 0)) \"pos\") (_ \"other\"))",
        )
        .unwrap();
        assert!(matches!(result, Value::String(ref s) if s == "pos"));
    }

    #[test]
    fn test_eval_match_guard_fails_falls_through() {
        // Guard false → fall through to next arm. Bindings from the failed
        // guard arm must not leak into the catch-all.
        let result = eval_str(
            "(match -3 ((guard x (> x 0)) \"pos\") (_ \"other\"))",
        )
        .unwrap();
        assert!(matches!(result, Value::String(ref s) if s == "other"));
    }

    #[test]
    fn test_eval_match_guard_with_cons_binding() {
        // Variables bound by the inner cons pattern are visible to the guard.
        let result = eval_str(
            "(match (list 1 2 3) \
               ((guard (cons h _) (> h 0)) h) \
               (_ -1))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(1)));
    }

    #[test]
    fn test_eval_match_guard_under_as() {
        // `as` inside a guard exposes both component (`x`) and whole (`whole`)
        // bindings to the guard expression and the body.
        let result = eval_str(
            "(match 5 ((guard (as x whole) (> whole 0)) x) (_ -1))",
        )
        .unwrap();
        assert!(matches!(result, Value::Integer32(5)));
    }

    #[test]
    fn test_eval_match_guard_three_way() {
        // Classic guard use case: split positive / zero / negative in one match.
        let pos = eval_str(
            "(match 7 \
               ((guard x (> x 0)) \"positive\") \
               (0 \"zero\") \
               (_ \"negative\"))",
        )
        .unwrap();
        assert!(matches!(pos, Value::String(ref s) if s == "positive"));

        let zero = eval_str(
            "(match 0 \
               ((guard x (> x 0)) \"positive\") \
               (0 \"zero\") \
               (_ \"negative\"))",
        )
        .unwrap();
        assert!(matches!(zero, Value::String(ref s) if s == "zero"));

        let neg = eval_str(
            "(match -7 \
               ((guard x (> x 0)) \"positive\") \
               (0 \"zero\") \
               (_ \"negative\"))",
        )
        .unwrap();
        assert!(matches!(neg, Value::String(ref s) if s == "negative"));
    }

    #[test]
    fn test_type_check_match_guard_must_be_bool() {
        // Guard expression must be Bool. `(+ x 1)` is i32, so this should
        // be rejected by the type checker.
        let err = type_check_str(
            "(match 5 ((guard x (+ x 1)) \"x\") (_ \"y\"))",
        )
        .unwrap_err();
        assert!(err.contains("Bool"), "expected Bool error, got: {}", err);
    }

    #[test]
    fn test_type_check_match_guard_undefined_var() {
        // Free variable `y` inside the guard should be reported as undefined.
        let err = type_check_str(
            "(match 5 ((guard x (> y 0)) \"x\") (_ \"y\"))",
        )
        .unwrap_err();
        assert!(err.contains('y'), "expected undefined-var error, got: {}", err);
    }

    // ======================================================================
    // Exhaustiveness checking
    // ======================================================================

    #[test]
    fn test_exhaustive_bool_ok() {
        // Both true and false covered → exhaustive.
        let ty = type_check_str("(match (= 1 1) (true 1) (false 2))").unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_exhaustive_bool_missing_false() {
        // Only `true` arm; `false` is missing.
        let err = type_check_str("(match (= 1 1) (true 1))").unwrap_err();
        assert!(
            err.contains("not exhaustive") && err.contains("false"),
            "expected missing-false error, got: {}",
            err
        );
    }

    #[test]
    fn test_exhaustive_bool_wildcard_ok() {
        // Wildcard covers both bool values.
        let ty = type_check_str("(match (= 1 1) (true 1) (_ 2))").unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_exhaustive_list_ok() {
        // nil + (cons _ _) covers both list constructors.
        let ty = type_check_str(
            "(match (list 1 2) (nil 0) ((cons _ _) 1))",
        )
        .unwrap();
        assert_eq!(ty, Type::I32);
    }

    #[test]
    fn test_exhaustive_list_missing_nil() {
        // Only cons arm; nil is missing.
        let err = type_check_str("(match (list 1 2) ((cons _ _) 1))").unwrap_err();
        assert!(
            err.contains("not exhaustive") && err.contains("nil"),
            "expected missing-nil error, got: {}",
            err
        );
    }

    #[test]
    fn test_exhaustive_list_missing_cons() {
        // Only nil arm; cons is missing.
        let err = type_check_str("(match (list 1 2) (nil 0))").unwrap_err();
        assert!(
            err.contains("not exhaustive") && err.contains("cons"),
            "expected missing-cons error, got: {}",
            err
        );
    }

    #[test]
    fn test_exhaustive_nested_list_bool_partial() {
        // List<Bool> with nil and (cons true _) — missing (cons false _).
        let err = type_check_str(
            "(match (list true) (nil 0) ((cons true _) 1))",
        )
        .unwrap_err();
        assert!(
            err.contains("not exhaustive") && err.contains("cons") && err.contains("false"),
            "expected missing (cons false _) error, got: {}",
            err
        );
    }

    #[test]
    fn test_exhaustive_inferred_skipped() {
        // When the scrutinee type is Inferred (parameter type `_`), the
        // exhaustiveness check skips silently. This avoids false positives
        // before bidirectional inference (#8) lands. We use a lambda with
        // an inferred parameter type so the return type is also inferred —
        // the only thing under test is that the non-exhaustive `match`
        // inside does not raise an exhaustiveness error.
        let ty = type_check_str(
            "(fn [xs: _] (match xs ((cons _ _) 1)))",
        )
        .unwrap();
        assert!(matches!(ty, Type::Function { .. }));
    }

    #[test]
    fn test_exhaustive_guard_does_not_count() {
        // A guarded arm cannot satisfy exhaustiveness because its truth
        // value is only known at runtime. Even though `(guard true ...)`
        // structurally names `true`, the arm is not considered to cover it.
        let err = type_check_str(
            "(match (= 1 1) ((guard true (= 1 1)) 1) (false 2))",
        )
        .unwrap_err();
        assert!(
            err.contains("not exhaustive") && err.contains("true"),
            "expected missing-true error (guard does not count), got: {}",
            err
        );
    }
}
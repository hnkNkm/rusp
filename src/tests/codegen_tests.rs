#[cfg(test)]
mod tests {
    use crate::codegen;
    use crate::parser;

    #[test]
    fn smoke_module_emits_module_header() {
        // Step 1: prove that inkwell + LLVM toolchain are linked correctly.
        // The IR for an empty module still includes the `; ModuleID = ...`
        // header and a `source_filename` line, so non-empty is enough here.
        let ir = codegen::smoke_module();
        assert!(!ir.is_empty(), "expected non-empty IR, got empty string");
        assert!(
            ir.contains("rusp_smoke"),
            "expected module name in IR, got: {}",
            ir
        );
    }

    /// Helper: parse `input` and JIT-run it as i32. Each call gets a fresh
    /// Context (created inside `jit_eval_i32`), so tests are isolated.
    fn jit_i32(input: &str) -> Result<i32, String> {
        let ast = parser::parse(input).map_err(|e| e.to_string())?;
        codegen::jit_eval_i32(&ast)
    }

    fn jit_i64(input: &str) -> Result<i64, String> {
        let ast = parser::parse(input).map_err(|e| e.to_string())?;
        codegen::jit_eval_i64(&ast)
    }

    #[test]
    fn jit_integer_literal() {
        assert_eq!(jit_i32("42").unwrap(), 42);
        assert_eq!(jit_i32("-7").unwrap(), -7);
        assert_eq!(jit_i32("0").unwrap(), 0);
    }

    #[test]
    fn jit_i32_addition() {
        assert_eq!(jit_i32("(+ 1 2)").unwrap(), 3);
        assert_eq!(jit_i32("(+ 100 -50)").unwrap(), 50);
    }

    #[test]
    fn jit_i32_subtraction() {
        assert_eq!(jit_i32("(- 10 4)").unwrap(), 6);
        assert_eq!(jit_i32("(- 0 5)").unwrap(), -5);
    }

    #[test]
    fn jit_i32_multiplication() {
        assert_eq!(jit_i32("(* 6 7)").unwrap(), 42);
        assert_eq!(jit_i32("(* -3 4)").unwrap(), -12);
    }

    #[test]
    fn jit_i32_division() {
        assert_eq!(jit_i32("(/ 20 4)").unwrap(), 5);
        // Signed division — round toward zero.
        assert_eq!(jit_i32("(/ -20 3)").unwrap(), -6);
    }

    #[test]
    fn jit_i32_nested_arithmetic() {
        // Both `Expr::List` (operator-headed) and arithmetic recursion exercised.
        assert_eq!(jit_i32("(* (+ 1 2) 3)").unwrap(), 9);
        assert_eq!(jit_i32("(+ (* 2 3) (- 10 4))").unwrap(), 12);
    }

    #[test]
    fn jit_i32_variadic_left_fold() {
        // `(+ a b c)` is left-folded to `((a + b) + c)`.
        assert_eq!(jit_i32("(+ 1 2 3 4)").unwrap(), 10);
        assert_eq!(jit_i32("(- 100 10 20 30)").unwrap(), 40);
    }

    // ----------- Step 3: i64 arithmetic -----------

    #[test]
    fn jit_i64_literal() {
        // The parser picks i64 once a literal exceeds i32::MAX, so this
        // also exercises the parser → codegen handoff for i64.
        assert_eq!(
            jit_i64("9223372036854775807").unwrap(),
            i64::MAX
        );
    }

    #[test]
    fn jit_i64_addition() {
        // Both operands are outside i32 range, so the parser tags them as i64.
        // (Mixing i32 and i64 in one form is rejected by the type checker
        // and would also surface as a width-mismatch error here.)
        assert_eq!(
            jit_i64("(+ 100000000000 1000000000000)").unwrap(),
            1_100_000_000_000
        );
    }

    #[test]
    fn jit_i64_full_arithmetic() {
        // Each operand must individually exceed i32::MAX so the parser
        // tags it as i64. (Rusp has no `5_i64` suffix syntax yet —
        // separating those concerns is outside the LLVM-MVP scope.)
        assert_eq!(
            jit_i64("(- 5000000000 4000000000)").unwrap(),
            1_000_000_000
        );
        // Both factors > i32::MAX (~2.1e9), product fits comfortably in i64.
        assert_eq!(
            jit_i64("(* 3000000000 2500000000)").unwrap(),
            7_500_000_000_000_000_000
        );
        assert_eq!(
            jit_i64("(/ 9000000000 3000000000)").unwrap(),
            3
        );
    }

    #[test]
    fn jit_width_mismatch_is_caught() {
        // Asking for i32 when the body produced i64 surfaces a clean
        // error rather than silently truncating bits.
        let err = codegen::jit_eval_i32(
            &parser::parse("100000000000").unwrap()
        )
        .unwrap_err();
        assert!(
            err.contains("requested i32") && err.contains("produced i64"),
            "expected width mismatch error, got: {}",
            err
        );
    }

    // ----------- Step 4: bool, comparison, if, and/or/not -----------

    fn jit_bool(input: &str) -> Result<bool, String> {
        let ast = parser::parse(input).map_err(|e| e.to_string())?;
        codegen::jit_eval_bool(&ast)
    }

    #[test]
    fn jit_bool_literal() {
        assert!(jit_bool("true").unwrap());
        assert!(!jit_bool("false").unwrap());
    }

    #[test]
    fn jit_i32_comparison() {
        assert!(jit_bool("(= 1 1)").unwrap());
        assert!(!jit_bool("(= 1 2)").unwrap());
        assert!(jit_bool("(< 1 2)").unwrap());
        assert!(!jit_bool("(< 2 1)").unwrap());
        assert!(jit_bool("(> 5 3)").unwrap());
        assert!(jit_bool("(<= 3 3)").unwrap());
        assert!(jit_bool("(>= 4 3)").unwrap());
        // Negative numbers (signed comparison).
        assert!(jit_bool("(< -5 0)").unwrap());
    }

    #[test]
    fn jit_i64_comparison() {
        // Operands are both > i32::MAX so the parser tags them as i64,
        // exercising the SLT codegen path on i64.
        assert!(jit_bool("(< 3000000000 4000000000)").unwrap());
        assert!(!jit_bool("(= 3000000000 4000000000)").unwrap());
    }

    #[test]
    fn jit_if_simple() {
        assert_eq!(jit_i32("(if true 10 20)").unwrap(), 10);
        assert_eq!(jit_i32("(if false 10 20)").unwrap(), 20);
        // Condition that is itself a comparison.
        assert_eq!(jit_i32("(if (= 1 1) 10 20)").unwrap(), 10);
        assert_eq!(jit_i32("(if (< 5 1) 10 20)").unwrap(), 20);
    }

    #[test]
    fn jit_if_nested() {
        // Nested if exercises that emit_if captures the *end* block of
        // each arm, not the start, so phi sources stay correct.
        let src = "(if (< 0 1) (if (< 1 2) 1 2) 3)";
        assert_eq!(jit_i32(src).unwrap(), 1);
        let src2 = "(if (< 0 1) (if (> 1 2) 1 2) 3)";
        assert_eq!(jit_i32(src2).unwrap(), 2);
    }

    #[test]
    fn jit_if_returning_bool() {
        // The merge phi should be i1 here.
        assert!(jit_bool("(if true (= 1 1) (= 1 2))").unwrap());
        assert!(!jit_bool("(if false (= 1 1) (= 1 2))").unwrap());
    }

    #[test]
    fn jit_and_basic() {
        assert!(jit_bool("(and true true)").unwrap());
        assert!(!jit_bool("(and true false)").unwrap());
        assert!(!jit_bool("(and false true)").unwrap());
        assert!(!jit_bool("(and false false)").unwrap());
        // Variadic and: left-fold.
        assert!(jit_bool("(and true true true)").unwrap());
        assert!(!jit_bool("(and true true false)").unwrap());
    }

    #[test]
    fn jit_or_basic() {
        assert!(jit_bool("(or true false)").unwrap());
        assert!(jit_bool("(or false true)").unwrap());
        assert!(!jit_bool("(or false false)").unwrap());
        assert!(jit_bool("(or false false true)").unwrap());
    }

    #[test]
    fn jit_not_basic() {
        assert!(!jit_bool("(not true)").unwrap());
        assert!(jit_bool("(not false)").unwrap());
        assert!(jit_bool("(not (= 1 2))").unwrap());
    }

    #[test]
    fn jit_logical_with_comparison() {
        // Compose comparisons + logical ops in one expression.
        assert!(jit_bool("(and (< 1 2) (> 3 1))").unwrap());
        assert!(jit_bool("(or (= 1 2) (< 1 2))").unwrap());
        assert!(jit_bool("(not (and (< 1 2) (> 1 3)))").unwrap());
    }

    // ----------- Step 5: f64 arithmetic and comparison -----------

    fn jit_f64(input: &str) -> Result<f64, String> {
        let ast = parser::parse(input).map_err(|e| e.to_string())?;
        codegen::jit_eval_f64(&ast)
    }

    #[test]
    fn jit_f64_literal() {
        assert_eq!(jit_f64("1.5").unwrap(), 1.5);
        assert_eq!(jit_f64("0.0").unwrap(), 0.0);
        assert_eq!(jit_f64("-2.25").unwrap(), -2.25);
    }

    #[test]
    fn jit_f64_arithmetic() {
        assert_eq!(jit_f64("(+. 1.5 2.5)").unwrap(), 4.0);
        assert_eq!(jit_f64("(-. 5.0 1.5)").unwrap(), 3.5);
        assert_eq!(jit_f64("(*. 2.0 3.5)").unwrap(), 7.0);
        assert_eq!(jit_f64("(/. 7.0 2.0)").unwrap(), 3.5);
    }

    #[test]
    fn jit_f64_variadic_left_fold() {
        assert_eq!(jit_f64("(+. 1.0 2.0 3.0 4.0)").unwrap(), 10.0);
        assert_eq!(jit_f64("(*. 2.0 3.0 4.0)").unwrap(), 24.0);
    }

    #[test]
    fn jit_f64_nested() {
        assert_eq!(jit_f64("(*. (+. 1.0 2.0) 3.0)").unwrap(), 9.0);
        assert_eq!(jit_f64("(+. (*. 2.0 3.0) (-. 10.0 4.0))").unwrap(), 12.0);
    }

    #[test]
    fn jit_f64_comparison() {
        assert!(jit_bool("(= 1.5 1.5)").unwrap());
        assert!(!jit_bool("(= 1.5 2.5)").unwrap());
        assert!(jit_bool("(< 1.5 2.5)").unwrap());
        assert!(!jit_bool("(< 2.5 1.5)").unwrap());
        assert!(jit_bool("(> 3.0 1.0)").unwrap());
        assert!(jit_bool("(<= 1.5 1.5)").unwrap());
        assert!(jit_bool("(>= 2.0 1.0)").unwrap());
    }

    #[test]
    fn jit_f64_with_if() {
        // `if` returning f64 — phi merge on f64 type.
        assert_eq!(jit_f64("(if true 1.5 2.5)").unwrap(), 1.5);
        assert_eq!(jit_f64("(if false 1.5 2.5)").unwrap(), 2.5);
        // Condition uses float comparison.
        assert_eq!(jit_f64("(if (< 1.5 2.0) 10.0 20.0)").unwrap(), 10.0);
    }

    #[test]
    fn jit_int_op_on_float_is_error() {
        // `+` on floats is rejected by the type checker, but the codegen
        // path also rejects it defensively.
        let err = jit_f64("(+ 1.0 2.0)").unwrap_err();
        // The type checker fires first ("expected i32" or similar).
        assert!(!err.is_empty());
    }

    // ----------- Step 6: let-in -----------

    #[test]
    fn jit_let_in_basic_i32() {
        // (let x 5 x) → 5
        assert_eq!(jit_i32("(let x 5 x)").unwrap(), 5);
        // Body uses bound name in arithmetic.
        assert_eq!(jit_i32("(let x 10 (+ x 1))").unwrap(), 11);
    }

    #[test]
    fn jit_let_in_basic_f64() {
        assert_eq!(jit_f64("(let half 0.5 half)").unwrap(), 0.5);
        assert_eq!(jit_f64("(let r 2.0 (*. r r))").unwrap(), 4.0);
    }

    #[test]
    fn jit_let_in_basic_bool() {
        assert!(jit_bool("(let p true p)").unwrap());
        assert!(jit_bool("(let p false (not p))").unwrap());
    }

    #[test]
    fn jit_let_in_nested() {
        // Nested let — outer binding visible inside inner body.
        assert_eq!(jit_i32("(let x 3 (let y 4 (+ x y)))").unwrap(), 7);
        // Inner binding shadows outer; body uses inner.
        assert_eq!(jit_i32("(let x 1 (let x 99 x))").unwrap(), 99);
    }

    #[test]
    fn jit_let_in_shadowing_restores() {
        // After the inner let's body, `x` should refer to the outer binding
        // again. Achieved by computing `(+ inner outer)` where the outer x
        // is read after the inner let scope ends.
        // (let x 10 (+ (let x 1 x) x)) → (+ 1 10) = 11
        assert_eq!(
            jit_i32("(let x 10 (+ (let x 1 x) x))").unwrap(),
            11
        );
    }

    #[test]
    fn jit_let_in_with_if() {
        // Bound value used in both arms.
        assert_eq!(
            jit_i32("(let x 5 (if (< x 10) (* x 2) x))").unwrap(),
            10
        );
    }

    #[test]
    fn jit_let_in_value_can_use_outer_bindings() {
        // `value` can reference earlier let-bound names.
        assert_eq!(
            jit_i32("(let x 3 (let y (* x 2) y))").unwrap(),
            6
        );
    }

    #[test]
    fn jit_undefined_variable_is_error() {
        // No binding for `x` — codegen should reject cleanly.
        let err = jit_i32("x").unwrap_err();
        assert!(
            err.contains("undefined variable"),
            "expected undefined variable error, got: {}",
            err
        );
    }

    // ----------- Step 7: defn + Call + recursion -----------

    /// Helper: parse a multi-form program by repeatedly calling
    /// `parse_expr` and consuming each top-level S-expression. Each
    /// form is then handed to `jit_eval_*_program`.
    fn parse_program(input: &str) -> Vec<crate::ast::Expr> {
        use crate::parser::expr::parse_expr;
        let mut forms = Vec::new();
        let mut rest = input.trim();
        while !rest.is_empty() {
            let (remaining, expr) = parse_expr(rest).expect("parse_program parse");
            forms.push(expr);
            rest = remaining.trim();
        }
        forms
    }

    fn jit_i32_prog(input: &str) -> Result<i32, String> {
        let forms = parse_program(input);
        codegen::jit_eval_i32_program(&forms)
    }

    fn jit_bool_prog(input: &str) -> Result<bool, String> {
        let forms = parse_program(input);
        codegen::jit_eval_bool_program(&forms)
    }

    fn jit_f64_prog(input: &str) -> Result<f64, String> {
        let forms = parse_program(input);
        codegen::jit_eval_f64_program(&forms)
    }

    #[test]
    fn jit_defn_simple_call() {
        let src = r#"
            (defn double [x: i32] -> i32 (* x 2))
            (double 21)
        "#;
        assert_eq!(jit_i32_prog(src).unwrap(), 42);
    }

    #[test]
    fn jit_defn_two_params() {
        let src = r#"
            (defn add [a: i32 b: i32] -> i32 (+ a b))
            (add 3 4)
        "#;
        assert_eq!(jit_i32_prog(src).unwrap(), 7);
    }

    #[test]
    fn jit_defn_returns_bool() {
        let src = r#"
            (defn is-pos [x: i32] -> bool (> x 0))
            (is-pos 5)
        "#;
        assert!(jit_bool_prog(src).unwrap());
        let src2 = r#"
            (defn is-pos [x: i32] -> bool (> x 0))
            (is-pos -3)
        "#;
        assert!(!jit_bool_prog(src2).unwrap());
    }

    #[test]
    fn jit_defn_returns_f64() {
        let src = r#"
            (defn area [r: f64] -> f64 (*. r r))
            (area 3.0)
        "#;
        assert_eq!(jit_f64_prog(src).unwrap(), 9.0);
    }

    #[test]
    fn jit_defn_recursion_factorial() {
        // The classic test: a recursive function whose body calls itself.
        // The function value must be registered before the body is emitted.
        let src = r#"
            (defn fact [n: i32] -> i32
              (if (<= n 1) 1 (* n (fact (- n 1)))))
            (fact 5)
        "#;
        assert_eq!(jit_i32_prog(src).unwrap(), 120);
    }

    #[test]
    fn jit_defn_recursion_fib() {
        let src = r#"
            (defn fib [n: i32] -> i32
              (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))
            (fib 10)
        "#;
        assert_eq!(jit_i32_prog(src).unwrap(), 55);
    }

    #[test]
    fn jit_defn_multiple() {
        let src = r#"
            (defn double [x: i32] -> i32 (* x 2))
            (defn quad [x: i32] -> i32 (double (double x)))
            (quad 5)
        "#;
        assert_eq!(jit_i32_prog(src).unwrap(), 20);
    }

    #[test]
    fn jit_defn_with_let_in_body() {
        let src = r#"
            (defn sum-sq [a: i32 b: i32] -> i32
              (let aa (* a a)
                (let bb (* b b)
                  (+ aa bb))))
            (sum-sq 3 4)
        "#;
        assert_eq!(jit_i32_prog(src).unwrap(), 25);
    }

    #[test]
    fn jit_call_undefined_function_errors() {
        // No defn for `nope`; the call should surface a clean error.
        let err = jit_i32_prog("(nope 1 2)").unwrap_err();
        assert!(
            err.contains("undefined function") || err.contains("nope"),
            "expected undefined-function error, got: {}",
            err
        );
    }

    #[test]
    fn jit_call_wrong_arity_errors() {
        let src = r#"
            (defn id [x: i32] -> i32 x)
            (id 1 2)
        "#;
        let err = jit_i32_prog(src).unwrap_err();
        assert!(
            err.contains("expects") && err.contains("arguments"),
            "expected arity error, got: {}",
            err
        );
    }

    #[test]
    fn jit_unsupported_node_is_error_not_panic() {
        // Anything outside Step 6's scope must return Err so that the
        // future `--llvm` REPL surfaces a clean message instead of crashing.
        // String literals don't have a JIT representation yet.
        let err = jit_i32(r#""hello""#).unwrap_err();
        assert!(
            err.contains("not supported"),
            "expected unsupported error, got: {}",
            err
        );
    }
}

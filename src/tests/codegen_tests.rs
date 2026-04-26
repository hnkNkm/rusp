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

    #[test]
    fn jit_unsupported_node_is_error_not_panic() {
        // Anything outside Step 2's scope must return Err so that the
        // future `--llvm` REPL surfaces a clean message instead of crashing.
        let err = jit_i32("(if true 1 0)").unwrap_err();
        assert!(
            err.contains("not supported"),
            "expected unsupported error, got: {}",
            err
        );
    }
}

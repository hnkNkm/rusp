//! Minimal JIT entry point for Step 2: parse → type-check → codegen → run.
//!
//! Only i32 literals and i32 arithmetic (`+`, `-`, `*`, `/`) are supported
//! at this stage. Each call creates its own LLVM `Context` and module,
//! wraps the input expression in an anonymous `__expr() -> i32` function,
//! adds the module to a fresh execution engine, and looks up + calls the
//! function. This keeps lifetimes simple and tests well isolated.

use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::values::IntValue;

use crate::ast::Expr;

pub type JitError = String;

/// Compile and JIT-run `expr` as an `i32`-returning thunk.
///
/// `expr` must already have type `i32`; the caller is expected to have
/// run the type checker. Any unsupported AST node returns an error rather
/// than panicking, so callers can surface a clean message in the REPL.
pub fn jit_eval_i32(expr: &Expr) -> Result<i32, JitError> {
    let context = Context::create();
    let module = context.create_module("rusp_jit");
    let builder = context.create_builder();

    // (1) Define `__expr() -> i32`.
    let i32_t = context.i32_type();
    let fn_t = i32_t.fn_type(&[], false);
    let function = module.add_function("__expr", fn_t, None);
    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    // (2) Translate the user expression into a single i32 SSA value.
    let cg = ExprCg {
        context: &context,
        builder: &builder,
    };
    let value = cg.emit(expr)?;
    builder
        .build_return(Some(&value))
        .map_err(|e| format!("LLVM build_return failed: {}", e))?;

    // (3) JIT and run.
    let engine: ExecutionEngine =
        module
            .create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| format!("failed to create JIT execution engine: {}", e))?;

    // SAFETY: signature `() -> i32` matches what we just emitted, the
    // module is owned by `engine`, and we drop both before returning.
    let result = unsafe {
        let func = engine
            .get_function::<unsafe extern "C" fn() -> i32>("__expr")
            .map_err(|e| format!("failed to look up __expr: {}", e))?;
        func.call()
    };
    Ok(result)
}

/// Per-invocation codegen helper. Borrows the Context/Module/Builder so
/// that 'ctx flows through cleanly.
struct ExprCg<'ctx, 'a> {
    context: &'ctx Context,
    builder: &'a Builder<'ctx>,
}

impl<'ctx, 'a> ExprCg<'ctx, 'a> {
    /// Generate IR for `expr`, returning the resulting i32 SSA value.
    fn emit(&self, expr: &Expr) -> Result<IntValue<'ctx>, JitError> {
        match expr {
            Expr::Integer32(n) => {
                // i32 literal — sign-extending the i64 cast is fine for
                // negative values because const_int treats the source as a
                // sign-extended u64.
                Ok(self.context.i32_type().const_int(*n as u64, true))
            }

            // Operator forms `(op a b ...)` parse as `Expr::List` with the
            // operator symbol at position 0. We dispatch on the symbol and
            // hand off to `gen_arith`.
            Expr::List(exprs) if !exprs.is_empty() => {
                if let Expr::Symbol(op) = &exprs[0] {
                    match op.as_str() {
                        "+" | "-" | "*" | "/" => self.gen_arith(op, &exprs[1..]),
                        other => Err(format!(
                            "--llvm: operator `{}` not supported yet",
                            other
                        )),
                    }
                } else {
                    Err("--llvm: only operator-headed lists are supported in Step 2".to_string())
                }
            }

            // `Expr::Call` is what bare `(f x y)` parses to. For Step 2
            // we only handle the operator forms above, which always come
            // through `Expr::List`. Calls land here only for user-defined
            // functions, which Step 7 will enable.
            Expr::Call { .. } => {
                Err("--llvm: function calls are not supported yet (Step 7)".to_string())
            }

            other => Err(format!(
                "--llvm: AST node {:?} is not supported by the MVP yet",
                std::mem::discriminant(other)
            )),
        }
    }

    /// Generate `(op arg0 arg1 ...)` for binary i32 arithmetic. Variadic
    /// in source (`(+ 1 2 3)`) is left-folded.
    fn gen_arith(&self, op: &str, args: &[Expr]) -> Result<IntValue<'ctx>, JitError> {
        if args.len() < 2 {
            return Err(format!(
                "operator `{}` requires at least 2 arguments, got {}",
                op,
                args.len()
            ));
        }

        let mut acc = self.emit(&args[0])?;
        for arg in &args[1..] {
            let rhs = self.emit(arg)?;
            acc = match op {
                "+" => self
                    .builder
                    .build_int_add(acc, rhs, "addtmp")
                    .map_err(|e| format!("LLVM build_int_add failed: {}", e))?,
                "-" => self
                    .builder
                    .build_int_sub(acc, rhs, "subtmp")
                    .map_err(|e| format!("LLVM build_int_sub failed: {}", e))?,
                "*" => self
                    .builder
                    .build_int_mul(acc, rhs, "multmp")
                    .map_err(|e| format!("LLVM build_int_mul failed: {}", e))?,
                "/" => self
                    .builder
                    .build_int_signed_div(acc, rhs, "divtmp")
                    .map_err(|e| format!("LLVM build_int_signed_div failed: {}", e))?,
                _ => unreachable!("operator dispatch checked already"),
            };
        }
        Ok(acc)
    }
}

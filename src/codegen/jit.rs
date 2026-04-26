//! Minimal JIT entry point for Steps 2–3: parse → type-check → codegen → run.
//!
//! Supports i32/i64 literals and integer arithmetic (`+`, `-`, `*`, `/`).
//! The expression's leading operand picks the integer width; the type
//! checker has already validated that all operands share the same width.
//!
//! Each call creates its own LLVM `Context` and module, wraps the input
//! in an anonymous thunk (`__expr() -> T`), JITs it via `ExecutionEngine`,
//! and looks the function up by name. Per-call Context keeps lifetimes
//! simple and tests well isolated.

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
    let result = compile_and_run(&context, expr, IntKind::I32)?;
    // SAFETY of the cast: the JIT returned an i32 inside a u64 (we trampoline
    // through u64 to keep one signature for both widths). Truncation is safe.
    Ok(result as i32)
}

/// Compile and JIT-run `expr` as an `i64`-returning thunk.
pub fn jit_eval_i64(expr: &Expr) -> Result<i64, JitError> {
    let context = Context::create();
    let result = compile_and_run(&context, expr, IntKind::I64)?;
    Ok(result as i64)
}

/// Compile, JIT, and run `__expr` returning a single integer of the
/// requested width. The actual width affects only the function signature
/// and the truncation/sign-extension at the boundary; codegen for the
/// body itself is uniform across i32/i64.
fn compile_and_run(
    context: &Context,
    expr: &Expr,
    expected: IntKind,
) -> Result<u64, JitError> {
    let module = context.create_module("rusp_jit");
    let builder = context.create_builder();

    let ret_t = match expected {
        IntKind::I32 => context.i32_type(),
        IntKind::I64 => context.i64_type(),
    };
    let fn_t = ret_t.fn_type(&[], false);
    let function = module.add_function("__expr", fn_t, None);
    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    let cg = ExprCg { context, builder: &builder };
    let value = cg.emit(expr)?;

    // The codegen body picks the width from the leading operand. Reject
    // mismatches loudly here so callers debug a width problem at the
    // boundary rather than misreading bits.
    let body_kind = IntKind::from_int_value(&value)?;
    if body_kind != expected {
        return Err(format!(
            "JIT requested {} but expression produced {}",
            expected.name(),
            body_kind.name()
        ));
    }

    builder
        .build_return(Some(&value))
        .map_err(|e| format!("LLVM build_return failed: {}", e))?;

    let engine: ExecutionEngine = module
        .create_jit_execution_engine(OptimizationLevel::None)
        .map_err(|e| format!("failed to create JIT execution engine: {}", e))?;

    // SAFETY: We just emitted the function with the matching signature and
    // verified the body's width above. The module is owned by `engine`,
    // both are dropped at end of scope.
    let result = unsafe {
        match expected {
            IntKind::I32 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> i32>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                func.call() as u64
            }
            IntKind::I64 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> i64>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                func.call() as u64
            }
        }
    };
    Ok(result)
}

/// Width tag used to keep i32 and i64 from cross-talking. We don't try to
/// support implicit widening — the type checker forbids mixing widths in
/// one expression, so codegen mirrors that contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntKind {
    I32,
    I64,
}

impl IntKind {
    fn name(&self) -> &'static str {
        match self {
            IntKind::I32 => "i32",
            IntKind::I64 => "i64",
        }
    }

    fn from_int_value(v: &IntValue<'_>) -> Result<Self, JitError> {
        match v.get_type().get_bit_width() {
            32 => Ok(IntKind::I32),
            64 => Ok(IntKind::I64),
            other => Err(format!("unsupported integer width: i{}", other)),
        }
    }
}

/// Per-invocation codegen helper. Borrows the Context/Module/Builder so
/// that 'ctx flows through cleanly.
struct ExprCg<'ctx, 'a> {
    context: &'ctx Context,
    builder: &'a Builder<'ctx>,
}

impl<'ctx, 'a> ExprCg<'ctx, 'a> {
    /// Generate IR for `expr`, returning the resulting integer SSA value.
    /// Width (i32 vs i64) is determined by the first integer literal /
    /// operand in the form, matching the type checker's rules.
    fn emit(&self, expr: &Expr) -> Result<IntValue<'ctx>, JitError> {
        match expr {
            Expr::Integer32(n) => {
                // i32 literal — `const_int` reads the bits of the supplied
                // u64; passing `true` for sign_extend handles negatives.
                Ok(self.context.i32_type().const_int(*n as u64, true))
            }

            Expr::Integer64(n) => {
                Ok(self.context.i64_type().const_int(*n as u64, true))
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
                    Err("--llvm: only operator-headed lists are supported in Step 3".to_string())
                }
            }

            // `Expr::Call` is what bare `(f x y)` parses to. For now we
            // only handle the operator forms above, which always come
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

    /// Generate `(op arg0 arg1 ...)` for binary integer arithmetic.
    /// Variadic in source (`(+ 1 2 3)`) is left-folded. Width is taken
    /// from the first operand; subsequent operands must match. The type
    /// checker normally guarantees this, so a mismatch here is a bug
    /// rather than user error.
    fn gen_arith(&self, op: &str, args: &[Expr]) -> Result<IntValue<'ctx>, JitError> {
        if args.len() < 2 {
            return Err(format!(
                "operator `{}` requires at least 2 arguments, got {}",
                op,
                args.len()
            ));
        }

        let mut acc = self.emit(&args[0])?;
        let acc_width = acc.get_type().get_bit_width();
        for arg in &args[1..] {
            let rhs = self.emit(arg)?;
            if rhs.get_type().get_bit_width() != acc_width {
                return Err(format!(
                    "operator `{}`: operand width mismatch (i{} vs i{}) — \
                     this is a codegen invariant that the type checker should have caught",
                    op,
                    acc_width,
                    rhs.get_type().get_bit_width()
                ));
            }
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

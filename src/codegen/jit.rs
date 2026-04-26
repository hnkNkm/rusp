//! Minimal JIT entry point for Steps 2–4: parse → type-check → codegen → run.
//!
//! Supports:
//! - i32/i64 literals and integer arithmetic (`+`, `-`, `*`, `/`)
//! - bool literals
//! - integer comparison (`=`, `<`, `>`, `<=`, `>=`) — i32 and i64
//! - `if` (with phi-merge)
//! - `and`/`or`/`not` (short-circuit for `and`/`or`, xor for `not`)
//!
//! The integer-arithmetic / comparison forms infer their width from the
//! leading operand; the type checker has already enforced that every
//! operand of one form shares that width.
//!
//! Each call creates its own LLVM `Context` and module, wraps the input
//! in an anonymous thunk (`__expr() -> T`), JITs it via `ExecutionEngine`,
//! and looks the function up by name. Per-call Context keeps lifetimes
//! simple and tests well isolated.

use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::values::{FunctionValue, IntValue};
use inkwell::{IntPredicate};

use crate::ast::Expr;

pub type JitError = String;

/// Compile and JIT-run `expr` as an `i32`-returning thunk.
///
/// `expr` must already have type `i32`; the caller is expected to have
/// run the type checker. Any unsupported AST node returns an error rather
/// than panicking, so callers can surface a clean message in the REPL.
pub fn jit_eval_i32(expr: &Expr) -> Result<i32, JitError> {
    let context = Context::create();
    let result = compile_and_run(&context, expr, ReturnKind::I32)?;
    // SAFETY of the cast: the JIT returned an i32 inside a u64 (we trampoline
    // through u64 to keep one signature for both widths). Truncation is safe.
    Ok(result as i32)
}

/// Compile and JIT-run `expr` as an `i64`-returning thunk.
pub fn jit_eval_i64(expr: &Expr) -> Result<i64, JitError> {
    let context = Context::create();
    let result = compile_and_run(&context, expr, ReturnKind::I64)?;
    Ok(result as i64)
}

/// Compile and JIT-run `expr` as a `bool`-returning thunk.
///
/// LLVM `i1` is zero-extended to `u8` at the FFI boundary because the
/// C ABI does not specify a layout for `i1`; receiving it as `u8` and
/// comparing against zero is portable and is what every other inkwell
/// example does too.
pub fn jit_eval_bool(expr: &Expr) -> Result<bool, JitError> {
    let context = Context::create();
    let result = compile_and_run(&context, expr, ReturnKind::Bool)?;
    Ok(result != 0)
}

/// What the top-level thunk should return. Determines the function
/// signature we emit and the cast applied at the FFI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnKind {
    I32,
    I64,
    Bool,
}

impl ReturnKind {
    fn name(&self) -> &'static str {
        match self {
            ReturnKind::I32 => "i32",
            ReturnKind::I64 => "i64",
            ReturnKind::Bool => "bool",
        }
    }
}

/// Compile, JIT, and run `__expr` returning a single integer-shaped
/// value of the requested kind. The actual width affects only the
/// function signature and the truncation/zero-extension at the
/// boundary; codegen for the body itself dispatches on the value's
/// own LLVM type.
fn compile_and_run(
    context: &Context,
    expr: &Expr,
    expected: ReturnKind,
) -> Result<u64, JitError> {
    let module = context.create_module("rusp_jit");
    let builder = context.create_builder();

    let ret_t = match expected {
        ReturnKind::I32 => context.i32_type(),
        ReturnKind::I64 => context.i64_type(),
        ReturnKind::Bool => context.bool_type(),
    };
    let fn_t = ret_t.fn_type(&[], false);
    let function = module.add_function("__expr", fn_t, None);
    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    let cg = ExprCg { context, builder: &builder, function };
    let value = cg.emit(expr)?;

    // Width / kind validation: surface mismatches loudly so callers debug
    // a width problem at the boundary rather than misreading bits.
    let body_kind = ReturnKind::from_int_value(&value)?;
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
            ReturnKind::I32 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> i32>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                func.call() as u64
            }
            ReturnKind::I64 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> i64>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                func.call() as u64
            }
            ReturnKind::Bool => {
                // i1 → u8 at the boundary (see jit_eval_bool).
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> u8>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                func.call() as u64
            }
        }
    };
    Ok(result)
}

impl ReturnKind {
    /// Map the bit-width of the body's SSA value to the matching ReturnKind.
    /// i1 → Bool, i32 → I32, i64 → I64. Anything else is unsupported here.
    fn from_int_value(v: &IntValue<'_>) -> Result<Self, JitError> {
        match v.get_type().get_bit_width() {
            1 => Ok(ReturnKind::Bool),
            32 => Ok(ReturnKind::I32),
            64 => Ok(ReturnKind::I64),
            other => Err(format!("unsupported integer width: i{}", other)),
        }
    }
}

/// Per-invocation codegen helper. Borrows the Context/Module/Builder so
/// that 'ctx flows through cleanly. Carries the enclosing function so
/// `if` / short-circuit forms can append fresh basic blocks.
struct ExprCg<'ctx, 'a> {
    context: &'ctx Context,
    builder: &'a Builder<'ctx>,
    function: FunctionValue<'ctx>,
}

impl<'ctx, 'a> ExprCg<'ctx, 'a> {
    /// Generate IR for `expr`, returning the resulting integer SSA value
    /// (treating `bool` as `i1`).
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

            Expr::Bool(b) => {
                Ok(self.context.bool_type().const_int(u64::from(*b), false))
            }

            // `if` is its own AST node (not a List with "if" symbol).
            // Emit cond → conditional branch into then/else blocks → phi
            // in a merge block. This is the textbook lowering, taken
            // straight from the Kaleidoscope tutorial.
            Expr::If { condition, then_branch, else_branch } => {
                self.emit_if(condition, then_branch, else_branch)
            }

            // Operator forms `(op a b ...)` parse as `Expr::List` with the
            // operator symbol at position 0. Dispatch on the symbol.
            Expr::List(exprs) if !exprs.is_empty() => {
                if let Expr::Symbol(op) = &exprs[0] {
                    let args = &exprs[1..];
                    match op.as_str() {
                        "+" | "-" | "*" | "/" => self.gen_arith(op, args),
                        "=" | "<" | ">" | "<=" | ">=" => self.gen_cmp(op, args),
                        "and" => self.gen_and(args),
                        "or" => self.gen_or(args),
                        "not" => self.gen_not(args),
                        other => Err(format!(
                            "--llvm: operator `{}` not supported yet",
                            other
                        )),
                    }
                } else {
                    Err("--llvm: only operator-headed lists are supported in Step 4".to_string())
                }
            }

            // `Expr::Call` is what bare `(f x y)` parses to. For now we
            // only handle the operator forms above. Calls land here only
            // for user-defined functions, which Step 7 will enable.
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
        if acc_width != 32 && acc_width != 64 {
            return Err(format!(
                "operator `{}` requires integer operands, got i{}",
                op, acc_width
            ));
        }
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

    /// Generate a binary integer comparison `(op a b)`. Result is `i1`.
    /// Comparisons in Rusp are strictly binary (the type checker rejects
    /// arity != 2), so we don't try to fold variadic forms.
    fn gen_cmp(&self, op: &str, args: &[Expr]) -> Result<IntValue<'ctx>, JitError> {
        if args.len() != 2 {
            return Err(format!(
                "comparison `{}` requires exactly 2 arguments, got {}",
                op,
                args.len()
            ));
        }
        let lhs = self.emit(&args[0])?;
        let rhs = self.emit(&args[1])?;
        let lw = lhs.get_type().get_bit_width();
        let rw = rhs.get_type().get_bit_width();
        if lw != rw {
            return Err(format!(
                "comparison `{}`: operand width mismatch (i{} vs i{})",
                op, lw, rw
            ));
        }
        if lw != 32 && lw != 64 {
            return Err(format!(
                "comparison `{}` only supports integer operands (i32/i64), got i{}",
                op, lw
            ));
        }
        let pred = match op {
            "=" => IntPredicate::EQ,
            "<" => IntPredicate::SLT,
            ">" => IntPredicate::SGT,
            "<=" => IntPredicate::SLE,
            ">=" => IntPredicate::SGE,
            _ => unreachable!("comparison dispatch checked already"),
        };
        self.builder
            .build_int_compare(pred, lhs, rhs, "cmptmp")
            .map_err(|e| format!("LLVM build_int_compare failed: {}", e))
    }

    /// Lower `(if c t e)` with a phi at the merge.
    ///
    /// Layout:
    ///   ; current block evaluates `c`, then conditional branch
    ///   then_bb: <emit t>; br merge
    ///   else_bb: <emit e>; br merge
    ///   merge: %r = phi [t_val, then_end], [e_val, else_end]
    ///
    /// We capture the *end-of-arm* block (not the start) because each
    /// arm's emit can itself create new blocks (nested ifs etc.), and
    /// phi must reference the block that actually fell through to merge.
    fn emit_if(
        &self,
        cond: &Expr,
        then_e: &Expr,
        else_e: &Expr,
    ) -> Result<IntValue<'ctx>, JitError> {
        let cond_v = self.emit(cond)?;
        if cond_v.get_type().get_bit_width() != 1 {
            return Err(format!(
                "`if` condition must be bool, got i{}",
                cond_v.get_type().get_bit_width()
            ));
        }

        let then_bb = self.context.append_basic_block(self.function, "then");
        let else_bb = self.context.append_basic_block(self.function, "else");
        let merge_bb = self.context.append_basic_block(self.function, "ifcont");

        self.builder
            .build_conditional_branch(cond_v, then_bb, else_bb)
            .map_err(|e| format!("LLVM build_conditional_branch failed: {}", e))?;

        // then arm
        self.builder.position_at_end(then_bb);
        let then_v = self.emit(then_e)?;
        let then_end = self.builder.get_insert_block().expect("then arm has insert block");
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("LLVM build_unconditional_branch failed: {}", e))?;

        // else arm
        self.builder.position_at_end(else_bb);
        let else_v = self.emit(else_e)?;
        let else_end = self.builder.get_insert_block().expect("else arm has insert block");
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("LLVM build_unconditional_branch failed: {}", e))?;

        // Both arms must produce the same type. The type checker enforces
        // this; check defensively.
        if then_v.get_type() != else_v.get_type() {
            return Err(format!(
                "`if` branches have different types (i{} vs i{}) — \
                 the type checker should have caught this",
                then_v.get_type().get_bit_width(),
                else_v.get_type().get_bit_width()
            ));
        }

        // merge with phi
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(then_v.get_type(), "iftmp")
            .map_err(|e| format!("LLVM build_phi failed: {}", e))?;
        phi.add_incoming(&[(&then_v, then_end), (&else_v, else_end)]);
        Ok(phi.as_basic_value().into_int_value())
    }

    /// Short-circuit `and`. Variadic: `(and a b c)` is left-folded so
    /// that any false short-circuits to false without evaluating the
    /// rest. We implement each step as
    ///   if acc then (eval next) else false
    /// which the optimizer collapses to a chain of basic blocks anyway,
    /// so this stays simple.
    fn gen_and(&self, args: &[Expr]) -> Result<IntValue<'ctx>, JitError> {
        if args.is_empty() {
            // `(and)` is conventionally `true` in Lisp; Rusp's type
            // checker rejects this anyway, but keep behavior defined.
            return Ok(self.context.bool_type().const_int(1, false));
        }
        let mut acc = self.emit(&args[0])?;
        for arg in &args[1..] {
            acc = self.short_circuit(acc, arg, ShortCircuit::And)?;
        }
        Ok(acc)
    }

    /// Short-circuit `or`. Mirror of `gen_and`.
    fn gen_or(&self, args: &[Expr]) -> Result<IntValue<'ctx>, JitError> {
        if args.is_empty() {
            return Ok(self.context.bool_type().const_int(0, false));
        }
        let mut acc = self.emit(&args[0])?;
        for arg in &args[1..] {
            acc = self.short_circuit(acc, arg, ShortCircuit::Or)?;
        }
        Ok(acc)
    }

    /// Lower `acc OP rhs` as a control-flow short-circuit and phi.
    ///
    /// And: if acc then rhs else false
    /// Or:  if acc then true else rhs
    fn short_circuit(
        &self,
        acc: IntValue<'ctx>,
        rhs_expr: &Expr,
        kind: ShortCircuit,
    ) -> Result<IntValue<'ctx>, JitError> {
        if acc.get_type().get_bit_width() != 1 {
            return Err(format!(
                "`{}` operand must be bool, got i{}",
                kind.name(),
                acc.get_type().get_bit_width()
            ));
        }

        let bool_t = self.context.bool_type();
        let eval_bb = self.context.append_basic_block(self.function, kind.eval_label());
        let skip_bb = self.context.append_basic_block(self.function, kind.skip_label());
        let merge_bb = self.context.append_basic_block(self.function, "sccont");

        // Branch direction depends on the operator: for `and`, evaluate
        // rhs only if acc is true; for `or`, only if acc is false.
        let (then_target, else_target) = match kind {
            ShortCircuit::And => (eval_bb, skip_bb),
            ShortCircuit::Or => (skip_bb, eval_bb),
        };
        self.builder
            .build_conditional_branch(acc, then_target, else_target)
            .map_err(|e| format!("LLVM build_conditional_branch failed: {}", e))?;

        // Evaluation block: emit rhs and branch to merge.
        self.builder.position_at_end(eval_bb);
        let rhs = self.emit(rhs_expr)?;
        if rhs.get_type().get_bit_width() != 1 {
            return Err(format!(
                "`{}` operand must be bool, got i{}",
                kind.name(),
                rhs.get_type().get_bit_width()
            ));
        }
        let eval_end = self.builder.get_insert_block().expect("eval has insert block");
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("LLVM build_unconditional_branch failed: {}", e))?;

        // Skip block: short-circuit constant.
        self.builder.position_at_end(skip_bb);
        let short_v: IntValue<'ctx> = match kind {
            ShortCircuit::And => bool_t.const_int(0, false),
            ShortCircuit::Or => bool_t.const_int(1, false),
        };
        let skip_end = self.builder.get_insert_block().expect("skip has insert block");
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("LLVM build_unconditional_branch failed: {}", e))?;

        // Merge with phi.
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(bool_t, "sctmp")
            .map_err(|e| format!("LLVM build_phi failed: {}", e))?;
        phi.add_incoming(&[(&rhs, eval_end), (&short_v, skip_end)]);
        Ok(phi.as_basic_value().into_int_value())
    }

    /// `(not b)` → `b XOR 1`. No control flow needed since it always
    /// evaluates the operand.
    fn gen_not(&self, args: &[Expr]) -> Result<IntValue<'ctx>, JitError> {
        if args.len() != 1 {
            return Err(format!(
                "`not` requires exactly 1 argument, got {}",
                args.len()
            ));
        }
        let v = self.emit(&args[0])?;
        if v.get_type().get_bit_width() != 1 {
            return Err(format!(
                "`not` operand must be bool, got i{}",
                v.get_type().get_bit_width()
            ));
        }
        let one = self.context.bool_type().const_int(1, false);
        self.builder
            .build_xor(v, one, "nottmp")
            .map_err(|e| format!("LLVM build_xor failed: {}", e))
    }
}

/// Internal tag for `short_circuit` so it can pick branch direction and
/// short-circuit constant without two near-duplicate methods.
#[derive(Debug, Clone, Copy)]
enum ShortCircuit {
    And,
    Or,
}

impl ShortCircuit {
    fn name(&self) -> &'static str {
        match self {
            ShortCircuit::And => "and",
            ShortCircuit::Or => "or",
        }
    }
    fn eval_label(&self) -> &'static str {
        match self {
            ShortCircuit::And => "and_eval",
            ShortCircuit::Or => "or_eval",
        }
    }
    fn skip_label(&self) -> &'static str {
        match self {
            ShortCircuit::And => "and_skip",
            ShortCircuit::Or => "or_skip",
        }
    }
}

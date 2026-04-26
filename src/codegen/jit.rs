//! Minimal JIT entry point for Steps 2–8: parse → type-check → codegen → run.
//!
//! Supports:
//! - i32/i64 literals and integer arithmetic (`+`, `-`, `*`, `/`)
//! - f64 literals and float arithmetic (`+.`, `-.`, `*.`, `/.`)
//! - bool literals
//! - comparison (`=`, `<`, `>`, `<=`, `>=`) on i32, i64, and f64
//! - `if` (with phi-merge)
//! - `and`/`or`/`not` (short-circuit for `and`/`or`, xor for `not`)
//! - `let`-in (lexical bindings via a flat HashMap; SSA values, no alloca)
//! - `defn` + `(f x y)` calls, including direct recursion
//! - `(fn [...] body)` capture-free lambdas — bound via `let` and called by name
//!
//! Integer arithmetic / comparison forms infer their width from the
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
use inkwell::module::Module;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FloatValue, FunctionValue, IntValue,
};
use inkwell::{FloatPredicate, IntPredicate};

use std::cell::Cell;
use std::collections::HashMap;

use crate::ast::{Expr, Type};
use crate::types::{type_check, TypeEnv};

pub type JitError = String;

/// Compile and JIT-run `expr` as an `i32`-returning thunk.
///
/// `expr` must already have type `i32`; the caller is expected to have
/// run the type checker. Any unsupported AST node returns an error rather
/// than panicking, so callers can surface a clean message in the REPL.
pub fn jit_eval_i32(expr: &Expr) -> Result<i32, JitError> {
    jit_eval_i32_program(std::slice::from_ref(expr))
}

/// Compile and JIT-run `expr` as an `i64`-returning thunk.
pub fn jit_eval_i64(expr: &Expr) -> Result<i64, JitError> {
    jit_eval_i64_program(std::slice::from_ref(expr))
}

/// Compile and JIT-run `expr` as a `bool`-returning thunk.
///
/// LLVM `i1` is zero-extended to `u8` at the FFI boundary because the
/// C ABI does not specify a layout for `i1`; receiving it as `u8` and
/// comparing against zero is portable and is what every other inkwell
/// example does too.
pub fn jit_eval_bool(expr: &Expr) -> Result<bool, JitError> {
    jit_eval_bool_program(std::slice::from_ref(expr))
}

/// Compile and JIT-run `expr` as an `f64`-returning thunk. Floats can't
/// trampoline through u64 (different ABI registers, no bitcast at the
/// boundary), so the float case is handled separately.
pub fn jit_eval_f64(expr: &Expr) -> Result<f64, JitError> {
    jit_eval_f64_program(std::slice::from_ref(expr))
}

/// Program-form variants: any number of leading `defn` forms followed
/// by a final expression. The defns are emitted as real LLVM functions
/// (so they're available for `Call` and recursion); the final expression
/// becomes the body of the `__expr` thunk. Empty slices and slices that
/// don't end in an expression-shaped form are rejected.
pub fn jit_eval_i32_program(forms: &[Expr]) -> Result<i32, JitError> {
    let context = Context::create();
    let result = compile_program_and_run(&context, forms, ReturnKind::I32)?;
    Ok(result.as_u64 as i32)
}

pub fn jit_eval_i64_program(forms: &[Expr]) -> Result<i64, JitError> {
    let context = Context::create();
    let result = compile_program_and_run(&context, forms, ReturnKind::I64)?;
    Ok(result.as_u64 as i64)
}

pub fn jit_eval_bool_program(forms: &[Expr]) -> Result<bool, JitError> {
    let context = Context::create();
    let result = compile_program_and_run(&context, forms, ReturnKind::Bool)?;
    Ok(result.as_u64 != 0)
}

pub fn jit_eval_f64_program(forms: &[Expr]) -> Result<f64, JitError> {
    let context = Context::create();
    let result = compile_program_and_run(&context, forms, ReturnKind::F64)?;
    Ok(result.as_f64)
}

/// Split a program slice into `(leading defns, final expression)`.
/// Returns an error if the slice is empty or doesn't end in something
/// that can be the result expression. We don't enforce that all defns
/// must precede the expression — a `defn` after the result expression
/// would be unreachable in this single-thunk model.
fn split_program(forms: &[Expr]) -> Result<(Vec<&Expr>, &Expr), JitError> {
    if forms.is_empty() {
        return Err("--llvm: empty program (need at least one expression)".to_string());
    }
    let last_idx = forms.len() - 1;
    let last = &forms[last_idx];
    if matches!(last, Expr::Defn { .. }) {
        return Err(
            "--llvm: program's last form must be an expression, not a `defn`".to_string(),
        );
    }
    let mut defns: Vec<&Expr> = Vec::with_capacity(last_idx);
    for f in &forms[..last_idx] {
        if !matches!(f, Expr::Defn { .. }) {
            return Err(
                "--llvm: only leading `defn` forms followed by one expression are supported"
                    .to_string(),
            );
        }
        defns.push(f);
    }
    Ok((defns, last))
}

/// Emit a `defn` as a real LLVM function in `module`. Registers the
/// function value in `functions` *before* emitting the body so
/// recursive calls resolve. Parameters become entries in the body's
/// `env`. The body must produce a value matching the declared return
/// type.
pub(crate) fn emit_defn<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    functions: &mut HashMap<String, FunctionValue<'ctx>>,
    lambda_counter: &Cell<u32>,
    expr: &Expr,
) -> Result<(), JitError> {
    let Expr::Defn { name, params, return_type, body } = expr else {
        return Err("emit_defn called with non-Defn".to_string());
    };

    let param_tys: Vec<BasicMetadataTypeEnum> = params
        .iter()
        .map(|(_, ty)| -> Result<BasicMetadataTypeEnum, JitError> {
            Ok(type_to_basic(context, ty)?.into())
        })
        .collect::<Result<_, _>>()?;
    let ret_basic = type_to_basic(context, return_type)?;
    let fn_t = ret_basic.fn_type(&param_tys, false);
    let function = module.add_function(name, fn_t, None);
    functions.insert(name.clone(), function);

    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    // Bind params into the env.
    let mut env: HashMap<String, EmitVal<'ctx>> = HashMap::new();
    for (i, (pname, _)) in params.iter().enumerate() {
        let pv = function
            .get_nth_param(i as u32)
            .ok_or_else(|| format!("defn `{}`: missing param {}", name, i))?;
        env.insert(pname.clone(), basic_to_emit(pv)?);
    }

    let mut cg = ExprCg {
        context,
        module,
        builder,
        function,
        env,
        functions: functions.clone(),
        lambda_counter,
    };
    let body_val = cg.emit(body)?;

    // Body's kind must agree with the declared return type. FuncRef
    // can't appear here because its `as_basic_value_enum()` errors out.
    let body_basic = body_val.as_basic_value_enum().map_err(|e| {
        format!("defn `{}`: cannot return a function value from a defn body: {}", name, e)
    })?;
    if body_basic.get_type() != ret_basic {
        return Err(format!(
            "defn `{}`: body type {} doesn't match declared return type",
            name,
            body_val.type_name()
        ));
    }

    match body_val {
        EmitVal::Int(iv) => builder
            .build_return(Some(&iv as &dyn BasicValue))
            .map(|_| ())
            .map_err(|e| format!("LLVM build_return failed: {}", e))?,
        EmitVal::Float(fv) => builder
            .build_return(Some(&fv as &dyn BasicValue))
            .map(|_| ())
            .map_err(|e| format!("LLVM build_return failed: {}", e))?,
        EmitVal::FuncRef(_) => {
            return Err(format!(
                "defn `{}`: cannot return a function value from a defn body",
                name
            ));
        }
    };

    // Functions discovered while emitting this body (none for now,
    // since defns aren't nested) flow back via the cg.functions
    // copy. Merge new entries.
    for (k, v) in cg.functions {
        functions.entry(k).or_insert(v);
    }
    Ok(())
}

/// Map a Rusp `Type` to the LLVM basic type used for a function
/// parameter or return slot. Only scalar Rusp types are supported.
fn type_to_basic<'ctx>(context: &'ctx Context, ty: &Type) -> Result<BasicTypeEnum<'ctx>, JitError> {
    Ok(match ty {
        Type::I32 => context.i32_type().into(),
        Type::I64 => context.i64_type().into(),
        Type::Bool => context.bool_type().into(),
        Type::F64 => context.f64_type().into(),
        Type::String => return Err("--llvm: String type is not supported by the MVP".to_string()),
        Type::List(_) => return Err("--llvm: List type is not supported by the MVP".to_string()),
        Type::Function { .. } => {
            return Err("--llvm: first-class function types are not supported by the MVP".to_string());
        }
        Type::Inferred => {
            return Err(
                "--llvm: inferred type leaked to codegen — type checker should have resolved it"
                    .to_string(),
            );
        }
    })
}

/// Wrap a function param's `BasicValueEnum` as an `EmitVal`. Floats
/// become Float, all integer widths become Int.
fn basic_to_emit(v: BasicValueEnum<'_>) -> Result<EmitVal<'_>, JitError> {
    match v {
        BasicValueEnum::IntValue(iv) => Ok(EmitVal::Int(iv)),
        BasicValueEnum::FloatValue(fv) => Ok(EmitVal::Float(fv)),
        other => Err(format!(
            "--llvm: function param has unsupported LLVM type {:?}",
            other.get_type()
        )),
    }
}

/// What the top-level thunk should return. Determines the function
/// signature we emit and how the boundary value is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnKind {
    I32,
    I64,
    Bool,
    F64,
}

impl ReturnKind {
    fn name(&self) -> &'static str {
        match self {
            ReturnKind::I32 => "i32",
            ReturnKind::I64 => "i64",
            ReturnKind::Bool => "bool",
            ReturnKind::F64 => "f64",
        }
    }
}

/// Boundary value carrier — `compile_and_run` returns either an integer
/// (carried in u64) or a float (carried in f64). A union-style struct
/// works fine here because exactly one field is meaningful per call,
/// and `ReturnKind` tells the caller which.
#[derive(Clone, Copy)]
struct BoundaryValue {
    as_u64: u64,
    as_f64: f64,
}

impl BoundaryValue {
    fn from_u64(v: u64) -> Self {
        Self { as_u64: v, as_f64: 0.0 }
    }
    fn from_f64(v: f64) -> Self {
        Self { as_u64: 0, as_f64: v }
    }
}

/// Compile, JIT, and run a program: zero or more leading `defn` forms
/// followed by exactly one expression that becomes `__expr`'s body.
fn compile_program_and_run(
    context: &Context,
    forms: &[Expr],
    expected: ReturnKind,
) -> Result<BoundaryValue, JitError> {
    let (defns, expr) = split_program(forms)?;

    let module = context.create_module("rusp_jit");
    let builder = context.create_builder();

    // Emit each `defn` as a real LLVM function. The function is
    // registered in `functions` *before* its body is emitted so that
    // recursive self-calls resolve.
    let mut functions: HashMap<String, FunctionValue<'_>> = HashMap::new();
    let lambda_counter: Cell<u32> = Cell::new(0);
    for d in &defns {
        emit_defn(context, &module, &builder, &mut functions, &lambda_counter, d)?;
    }

    // Now emit the top-level thunk for the final expression.
    let ret_t: BasicTypeEnum = match expected {
        ReturnKind::I32 => context.i32_type().into(),
        ReturnKind::I64 => context.i64_type().into(),
        ReturnKind::Bool => context.bool_type().into(),
        ReturnKind::F64 => context.f64_type().into(),
    };
    let fn_t = ret_t.fn_type(&[], false);
    let function = module.add_function("__expr", fn_t, None);
    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    let mut cg = ExprCg {
        context,
        module: &module,
        builder: &builder,
        function,
        env: HashMap::new(),
        functions,
        lambda_counter: &lambda_counter,
    };
    let value = cg.emit(expr)?;

    // Width / kind validation: surface mismatches loudly so callers debug
    // a kind problem at the boundary rather than misreading bits.
    let body_kind = value.return_kind()?;
    if body_kind != expected {
        return Err(format!(
            "JIT requested {} but expression produced {}",
            expected.name(),
            body_kind.name()
        ));
    }

    match value {
        EmitVal::Int(iv) => builder
            .build_return(Some(&iv as &dyn BasicValue))
            .map(|_| ())
            .map_err(|e| format!("LLVM build_return failed: {}", e))?,
        EmitVal::Float(fv) => builder
            .build_return(Some(&fv as &dyn BasicValue))
            .map(|_| ())
            .map_err(|e| format!("LLVM build_return failed: {}", e))?,
        EmitVal::FuncRef(_) => {
            // Already rejected by `return_kind()` above; this is defensive.
            return Err(
                "--llvm: top-level expression evaluates to a function value".to_string(),
            );
        }
    };

    let engine: ExecutionEngine = module
        .create_jit_execution_engine(OptimizationLevel::None)
        .map_err(|e| format!("failed to create JIT execution engine: {}", e))?;

    // SAFETY: We just emitted the function with the matching signature and
    // verified the body's kind above. The module is owned by `engine`,
    // both are dropped at end of scope.
    let result = unsafe {
        match expected {
            ReturnKind::I32 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> i32>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                BoundaryValue::from_u64(func.call() as u64)
            }
            ReturnKind::I64 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> i64>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                BoundaryValue::from_u64(func.call() as u64)
            }
            ReturnKind::Bool => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> u8>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                BoundaryValue::from_u64(func.call() as u64)
            }
            ReturnKind::F64 => {
                let func = engine
                    .get_function::<unsafe extern "C" fn() -> f64>("__expr")
                    .map_err(|e| format!("failed to look up __expr: {}", e))?;
                BoundaryValue::from_f64(func.call())
            }
        }
    };
    Ok(result)
}

/// SSA value produced by `emit`. We split int and float because LLVM's
/// arithmetic / comparison instructions are typed and the dispatch is
/// cleaner here than re-classifying via `BasicValueEnum` everywhere.
///
/// `FuncRef` is a *symbolic* reference to an LLVM function. Capture-
/// free lambdas produce a FuncRef that can flow through `let` and
/// land in a Call's head position. FuncRefs don't have a runtime
/// representation: returning one from the top-level thunk is an error.
#[derive(Clone, Copy)]
enum EmitVal<'ctx> {
    Int(IntValue<'ctx>),
    Float(FloatValue<'ctx>),
    FuncRef(FunctionValue<'ctx>),
}

impl<'ctx> EmitVal<'ctx> {
    fn return_kind(&self) -> Result<ReturnKind, JitError> {
        match self {
            EmitVal::Int(iv) => match iv.get_type().get_bit_width() {
                1 => Ok(ReturnKind::Bool),
                32 => Ok(ReturnKind::I32),
                64 => Ok(ReturnKind::I64),
                other => Err(format!("unsupported integer width: i{}", other)),
            },
            EmitVal::Float(_) => Ok(ReturnKind::F64),
            EmitVal::FuncRef(_) => Err(
                "--llvm: top-level expression evaluates to a function value, \
                 which has no runtime representation in this JIT"
                    .to_string(),
            ),
        }
    }

    /// Friendly name for error messages.
    fn type_name(&self) -> &'static str {
        match self {
            EmitVal::Int(iv) => match iv.get_type().get_bit_width() {
                1 => "bool",
                32 => "i32",
                64 => "i64",
                _ => "int(?)",
            },
            EmitVal::Float(_) => "f64",
            EmitVal::FuncRef(_) => "fn",
        }
    }

    /// Convert to a BasicValueEnum where possible. FuncRefs have no
    /// such representation, so callers that rely on this must already
    /// have rejected FuncRefs.
    fn as_basic_value_enum(&self) -> Result<BasicValueEnum<'ctx>, JitError> {
        match self {
            EmitVal::Int(iv) => Ok((*iv).into()),
            EmitVal::Float(fv) => Ok((*fv).into()),
            EmitVal::FuncRef(_) => Err(
                "--llvm: function reference cannot appear here as a value".to_string(),
            ),
        }
    }
}

/// Per-invocation codegen helper. Borrows the Context/Module/Builder so
/// that 'ctx flows through cleanly. Carries the enclosing function so
/// `if` / short-circuit forms can append fresh basic blocks.
struct ExprCg<'ctx, 'a> {
    context: &'ctx Context,
    /// The LLVM module we're emitting into. Lambda emission needs it
    /// to register a new `FunctionValue`; arithmetic / control flow
    /// don't, but the cost of carrying it everywhere is trivial.
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    function: FunctionValue<'ctx>,
    /// Lexical environment for `let`-bound names and function
    /// parameters. Flat (no parent chain): rebinding via shadowing is
    /// implemented by snapshotting the displaced entry and restoring
    /// it after the body. SSA-style — each binding maps to a value,
    /// not an alloca.
    env: HashMap<String, EmitVal<'ctx>>,
    /// User-defined functions (from `defn`) by name. Includes the
    /// currently-being-emitted function before its body is processed,
    /// so recursive calls resolve.
    functions: HashMap<String, FunctionValue<'ctx>>,
    /// Shared counter for `__lambda_N` name generation. A `Cell` so
    /// nested emits can bump it without re-borrowing `&mut self`.
    lambda_counter: &'a Cell<u32>,
}

impl<'ctx, 'a> ExprCg<'ctx, 'a> {
    /// Generate IR for `expr`, returning the resulting SSA value.
    fn emit(&mut self, expr: &Expr) -> Result<EmitVal<'ctx>, JitError> {
        match expr {
            Expr::Integer32(n) => Ok(EmitVal::Int(
                self.context.i32_type().const_int(*n as u64, true),
            )),

            Expr::Integer64(n) => Ok(EmitVal::Int(
                self.context.i64_type().const_int(*n as u64, true),
            )),

            Expr::Float(n) => Ok(EmitVal::Float(self.context.f64_type().const_float(*n))),

            Expr::Bool(b) => Ok(EmitVal::Int(
                self.context.bool_type().const_int(u64::from(*b), false),
            )),

            // Lexical lookup. Operator symbols (e.g. `+`, `<`) never
            // reach here because they appear as the head of a List
            // and are handled there; only `let`-bound user names do.
            Expr::Symbol(name) => self.env.get(name).copied().ok_or_else(|| {
                format!("--llvm: undefined variable `{}`", name)
            }),

            // `let-in`: bind value, emit body in extended env, restore.
            // Top-level `let` (no body) doesn't make sense for a
            // single-thunk JIT and is rejected here.
            Expr::Let { name, value, body, .. } => self.emit_let(name, value, body.as_deref()),

            // `(fn [params] -> ret body)` — a capture-free anonymous
            // function. We emit it as a real LLVM function with a
            // synthetic name (`__lambda_N`) and return a `FuncRef`
            // that can flow through `let` and land at a call site.
            // The body is emitted with an env containing only the
            // lambda's params (no enclosing-scope captures).
            Expr::Lambda { params, return_type, body } => {
                self.emit_lambda(params, return_type.as_ref(), body)
            }

            // `if` is its own AST node (not a List with "if" symbol).
            Expr::If { condition, then_branch, else_branch } => {
                self.emit_if(condition, then_branch, else_branch)
            }

            // S-expression forms `(head a b ...)` parse as `Expr::List`.
            // The head determines what to do: built-in operators get
            // dedicated codegen; anything else is treated as a user-
            // defined function call resolved via `self.functions`.
            // (The parser doesn't produce `Expr::Call`; that variant
            // exists in the AST but isn't emitted from S-expressions.)
            Expr::List(exprs) if !exprs.is_empty() => {
                if let Expr::Symbol(op) = &exprs[0] {
                    let args = &exprs[1..];
                    match op.as_str() {
                        "+" | "-" | "*" | "/" => self.gen_int_arith(op, args),
                        "+." | "-." | "*." | "/." => self.gen_float_arith(op, args),
                        "=" | "<" | ">" | "<=" | ">=" => self.gen_cmp(op, args),
                        "and" => self.gen_and(args),
                        "or" => self.gen_or(args),
                        "not" => self.gen_not(args),
                        // Fall through to user-function call.
                        _ => self.emit_user_call(op, args),
                    }
                } else {
                    Err("--llvm: only symbol-headed lists are supported".to_string())
                }
            }

            // `Expr::Call` is in the AST but the current parser never
            // produces it; if it ever does, route through emit_user_call.
            Expr::Call { func, args } => match func.as_ref() {
                Expr::Symbol(name) => self.emit_user_call(name, args),
                _ => Err("--llvm: only direct function calls are supported".to_string()),
            },

            other => Err(format!(
                "--llvm: AST node {:?} is not supported by the MVP yet",
                std::mem::discriminant(other)
            )),
        }
    }

    /// Generate `(op arg0 arg1 ...)` for binary integer arithmetic.
    /// Variadic in source (`(+ 1 2 3)`) is left-folded. Width is taken
    /// from the first operand.
    fn gen_int_arith(&mut self, op: &str, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        if args.len() < 2 {
            return Err(format!(
                "operator `{}` requires at least 2 arguments, got {}",
                op,
                args.len()
            ));
        }
        let first = self.emit(&args[0])?;
        let mut acc = self.expect_int(&first, op)?;
        let acc_width = acc.get_type().get_bit_width();
        if acc_width != 32 && acc_width != 64 {
            return Err(format!(
                "operator `{}` requires i32/i64 operands, got i{}",
                op, acc_width
            ));
        }
        for arg in &args[1..] {
            let rhs_v = self.emit(arg)?;
            let rhs = self.expect_int(&rhs_v, op)?;
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
        Ok(EmitVal::Int(acc))
    }

    /// Generate `(op. arg0 arg1 ...)` for binary float arithmetic.
    /// Mirrors `gen_int_arith`. Rusp's float ops are syntactically
    /// distinct from int ones (`+.` vs `+`), so the type checker has
    /// already guaranteed every operand is f64.
    fn gen_float_arith(&mut self, op: &str, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        if args.len() < 2 {
            return Err(format!(
                "operator `{}` requires at least 2 arguments, got {}",
                op,
                args.len()
            ));
        }
        let first = self.emit(&args[0])?;
        let mut acc = self.expect_float(&first, op)?;
        for arg in &args[1..] {
            let rhs_v = self.emit(arg)?;
            let rhs = self.expect_float(&rhs_v, op)?;
            acc = match op {
                "+." => self
                    .builder
                    .build_float_add(acc, rhs, "faddtmp")
                    .map_err(|e| format!("LLVM build_float_add failed: {}", e))?,
                "-." => self
                    .builder
                    .build_float_sub(acc, rhs, "fsubtmp")
                    .map_err(|e| format!("LLVM build_float_sub failed: {}", e))?,
                "*." => self
                    .builder
                    .build_float_mul(acc, rhs, "fmultmp")
                    .map_err(|e| format!("LLVM build_float_mul failed: {}", e))?,
                "/." => self
                    .builder
                    .build_float_div(acc, rhs, "fdivtmp")
                    .map_err(|e| format!("LLVM build_float_div failed: {}", e))?,
                _ => unreachable!("operator dispatch checked already"),
            };
        }
        Ok(EmitVal::Float(acc))
    }

    /// Generate a binary comparison `(op a b)` → `i1`.
    /// Dispatches on the LHS's value kind: integers use signed
    /// predicates, floats use ordered predicates (OEQ/OLT/...).
    /// "Ordered" means NaN compares false, matching the interpreter's
    /// IEEE-754 semantics.
    fn gen_cmp(&mut self, op: &str, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        if args.len() != 2 {
            return Err(format!(
                "comparison `{}` requires exactly 2 arguments, got {}",
                op,
                args.len()
            ));
        }
        let lhs = self.emit(&args[0])?;
        let rhs = self.emit(&args[1])?;
        match (lhs, rhs) {
            (EmitVal::Int(l), EmitVal::Int(r)) => {
                let lw = l.get_type().get_bit_width();
                let rw = r.get_type().get_bit_width();
                if lw != rw {
                    return Err(format!(
                        "comparison `{}`: operand width mismatch (i{} vs i{})",
                        op, lw, rw
                    ));
                }
                if lw != 32 && lw != 64 {
                    return Err(format!(
                        "comparison `{}` only supports i32/i64 integer operands, got i{}",
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
                let r = self
                    .builder
                    .build_int_compare(pred, l, r, "cmptmp")
                    .map_err(|e| format!("LLVM build_int_compare failed: {}", e))?;
                Ok(EmitVal::Int(r))
            }
            (EmitVal::Float(l), EmitVal::Float(r)) => {
                let pred = match op {
                    "=" => FloatPredicate::OEQ,
                    "<" => FloatPredicate::OLT,
                    ">" => FloatPredicate::OGT,
                    "<=" => FloatPredicate::OLE,
                    ">=" => FloatPredicate::OGE,
                    _ => unreachable!("comparison dispatch checked already"),
                };
                let r = self
                    .builder
                    .build_float_compare(pred, l, r, "fcmptmp")
                    .map_err(|e| format!("LLVM build_float_compare failed: {}", e))?;
                Ok(EmitVal::Int(r))
            }
            (l, r) => Err(format!(
                "comparison `{}`: cannot compare {} with {}",
                op,
                l.type_name(),
                r.type_name()
            )),
        }
    }

    /// Lower `(if c t e)` with a phi at the merge.
    fn emit_if(
        &mut self,
        cond: &Expr,
        then_e: &Expr,
        else_e: &Expr,
    ) -> Result<EmitVal<'ctx>, JitError> {
        let cond_v = self.emit(cond)?;
        let cond_int = match cond_v {
            EmitVal::Int(iv) if iv.get_type().get_bit_width() == 1 => iv,
            other => {
                return Err(format!(
                    "`if` condition must be bool, got {}",
                    other.type_name()
                ));
            }
        };

        let then_bb = self.context.append_basic_block(self.function, "then");
        let else_bb = self.context.append_basic_block(self.function, "else");
        let merge_bb = self.context.append_basic_block(self.function, "ifcont");

        self.builder
            .build_conditional_branch(cond_int, then_bb, else_bb)
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

        // Both arms must produce the same kind. The type checker enforces
        // this; check defensively. FuncRef can't go through phi (no
        // basic-value representation), so we reject it here.
        let then_be = then_v
            .as_basic_value_enum()
            .map_err(|e| format!("`if` then-branch: {}", e))?;
        let else_be = else_v
            .as_basic_value_enum()
            .map_err(|e| format!("`if` else-branch: {}", e))?;
        if then_be.get_type() != else_be.get_type() {
            return Err(format!(
                "`if` branches have different types ({} vs {}) — \
                 the type checker should have caught this",
                then_v.type_name(),
                else_v.type_name()
            ));
        }

        // merge with phi
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(then_be.get_type(), "iftmp")
            .map_err(|e| format!("LLVM build_phi failed: {}", e))?;
        phi.add_incoming(&[(&then_be, then_end), (&else_be, else_end)]);

        let merged = phi.as_basic_value();
        Ok(match (then_v, else_v) {
            (EmitVal::Int(_), _) => EmitVal::Int(merged.into_int_value()),
            (EmitVal::Float(_), _) => EmitVal::Float(merged.into_float_value()),
            (EmitVal::FuncRef(_), _) => {
                // Unreachable: `as_basic_value_enum` above would have
                // erred on FuncRef.
                return Err(
                    "--llvm: `if` branches cannot produce function values".to_string(),
                );
            }
        })
    }

    /// `(name arg1 arg2 ...)` — call a user-defined function. The
    /// callee must resolve to a name registered in `self.functions`.
    /// Since the parser produces all calls through `Expr::List`, we
    /// take the name as a `&str` directly.
    fn emit_user_call(&mut self, name: &str, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        // Resolve the callee. We look first at user `defn`s, then at
        // the lexical env for `let`-bound `FuncRef`s (capture-free
        // lambdas land here). User names are kept distinct from
        // builtin operators by the dispatch in `emit`.
        let callee = if let Some(fv) = self.functions.get(name).copied() {
            fv
        } else if let Some(EmitVal::FuncRef(fv)) = self.env.get(name).copied() {
            fv
        } else {
            return Err(format!("--llvm: undefined function `{}`", name));
        };

        let expected_arity = callee.count_params() as usize;
        if args.len() != expected_arity {
            return Err(format!(
                "--llvm: `{}` expects {} arguments, got {}",
                name,
                expected_arity,
                args.len()
            ));
        }

        let mut arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::with_capacity(args.len());
        for a in args {
            let v = self.emit(a)?;
            arg_vals.push(match v {
                EmitVal::Int(iv) => iv.into(),
                EmitVal::Float(fv) => fv.into(),
                EmitVal::FuncRef(_) => {
                    return Err(format!(
                        "--llvm: cannot pass a function value as an argument to `{}`",
                        name
                    ));
                }
            });
        }

        let call_site = self
            .builder
            .build_call(callee, &arg_vals, "calltmp")
            .map_err(|e| format!("LLVM build_call failed: {}", e))?;
        let ret = call_site
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("--llvm: `{}` returns void; can't use as a value", name))?;
        basic_to_emit(ret)
    }

    /// `(let name value body)` — let-in. Bind `name` to the value of
    /// `value`, emit `body` with that binding visible, then restore the
    /// previous binding (or remove it). Top-level `let` (no body) is
    /// rejected here because the JIT compiles a single expression.
    ///
    /// Shadowing is supported: if `name` already exists, save the old
    /// value before inserting and put it back after the body.
    fn emit_let(
        &mut self,
        name: &str,
        value: &Expr,
        body: Option<&Expr>,
    ) -> Result<EmitVal<'ctx>, JitError> {
        let body = body.ok_or_else(|| {
            "--llvm: top-level `let` (without body) is not supported in JIT mode".to_string()
        })?;
        let val = self.emit(value)?;
        let prev = self.env.insert(name.to_string(), val);
        let result = self.emit(body);
        match prev {
            Some(old) => {
                self.env.insert(name.to_string(), old);
            }
            None => {
                self.env.remove(name);
            }
        }
        result
    }

    /// `(fn [params] -> ret body)` — emit an anonymous LLVM function
    /// and return a `FuncRef` to it. The lambda is emitted as a
    /// top-level module function with a synthetic name (`__lambda_N`).
    /// It is **capture-free**: the body cannot see any enclosing
    /// `let`-bound names — that would require closure conversion,
    /// which is out of scope for the MVP. This is enforced implicitly
    /// because we build a fresh env containing only the params.
    ///
    /// The lambda's body is emitted *before* we return to the
    /// enclosing emit, but we save and restore the builder position
    /// so the enclosing function's IR is untouched.
    ///
    /// If the user omitted the return type (`(fn [x: i32] (* x 2))`),
    /// we run a small type-check pass on the body in a fresh env
    /// containing only the params, and use the inferred type. Since
    /// the body is capture-free, the params are sufficient context.
    /// (Once closures are added, this needs the enclosing TypeEnv.)
    fn emit_lambda(
        &mut self,
        params: &[(String, Type)],
        return_type: Option<&Type>,
        body: &Expr,
    ) -> Result<EmitVal<'ctx>, JitError> {
        // Resolve the return type — declared, or inferred via a small
        // type-check pass over the body. The pass is cheap (body is
        // capture-free, so the env never grows beyond the params).
        let inferred_ret;
        let ret_ty: &Type = match return_type {
            Some(t) => t,
            None => {
                let mut tenv = TypeEnv::new();
                for (pname, pty) in params {
                    tenv.insert(pname.clone(), pty.clone());
                }
                inferred_ret = type_check(body, &mut tenv).map_err(|e| {
                    format!(
                        "--llvm: failed to infer lambda return type: {}",
                        e
                    )
                })?;
                &inferred_ret
            }
        };

        // Build the LLVM function signature.
        let param_tys: Vec<BasicMetadataTypeEnum> = params
            .iter()
            .map(|(_, ty)| -> Result<BasicMetadataTypeEnum, JitError> {
                Ok(type_to_basic(self.context, ty)?.into())
            })
            .collect::<Result<_, _>>()?;
        let ret_basic = type_to_basic(self.context, ret_ty)?;
        let fn_t = ret_basic.fn_type(&param_tys, false);

        // Generate a fresh name. Module-unique by counter.
        let id = self.lambda_counter.get();
        self.lambda_counter.set(id + 1);
        let lambda_name = format!("__lambda_{}", id);
        let lambda_fv = self.module.add_function(&lambda_name, fn_t, None);

        // Save builder position so we can restore after emitting body.
        let saved_block = self.builder.get_insert_block();

        // Emit body in a fresh env with only the lambda's params.
        let entry = self.context.append_basic_block(lambda_fv, "entry");
        self.builder.position_at_end(entry);

        let mut lambda_env: HashMap<String, EmitVal<'ctx>> = HashMap::new();
        for (i, (pname, _)) in params.iter().enumerate() {
            let pv = lambda_fv
                .get_nth_param(i as u32)
                .ok_or_else(|| format!("lambda `{}`: missing param {}", lambda_name, i))?;
            lambda_env.insert(pname.clone(), basic_to_emit(pv)?);
        }

        let mut inner = ExprCg {
            context: self.context,
            module: self.module,
            builder: self.builder,
            function: lambda_fv,
            env: lambda_env,
            functions: self.functions.clone(),
            lambda_counter: self.lambda_counter,
        };
        let body_val = inner.emit(body)?;

        let body_basic = body_val.as_basic_value_enum().map_err(|e| {
            format!("lambda `{}`: {}", lambda_name, e)
        })?;
        if body_basic.get_type() != ret_basic {
            return Err(format!(
                "lambda `{}`: body type {} doesn't match declared return type",
                lambda_name,
                body_val.type_name()
            ));
        }
        match body_val {
            EmitVal::Int(iv) => self
                .builder
                .build_return(Some(&iv as &dyn BasicValue))
                .map(|_| ())
                .map_err(|e| format!("LLVM build_return failed: {}", e))?,
            EmitVal::Float(fv) => self
                .builder
                .build_return(Some(&fv as &dyn BasicValue))
                .map(|_| ())
                .map_err(|e| format!("LLVM build_return failed: {}", e))?,
            EmitVal::FuncRef(_) => {
                return Err(format!(
                    "lambda `{}`: cannot return a function value",
                    lambda_name
                ));
            }
        };

        // Restore builder position so the enclosing emitter continues
        // appending to the right block.
        if let Some(bb) = saved_block {
            self.builder.position_at_end(bb);
        }

        Ok(EmitVal::FuncRef(lambda_fv))
    }

    /// Short-circuit `and`. Variadic: `(and a b c)` is left-folded so
    /// that any false short-circuits to false without evaluating the rest.
    fn gen_and(&mut self, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        if args.is_empty() {
            return Ok(EmitVal::Int(self.context.bool_type().const_int(1, false)));
        }
        let first = self.emit(&args[0])?;
        let mut acc = self.expect_bool(&first, "and")?;
        for arg in &args[1..] {
            acc = self.short_circuit(acc, arg, ShortCircuit::And)?;
        }
        Ok(EmitVal::Int(acc))
    }

    /// Short-circuit `or`. Mirror of `gen_and`.
    fn gen_or(&mut self, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        if args.is_empty() {
            return Ok(EmitVal::Int(self.context.bool_type().const_int(0, false)));
        }
        let first = self.emit(&args[0])?;
        let mut acc = self.expect_bool(&first, "or")?;
        for arg in &args[1..] {
            acc = self.short_circuit(acc, arg, ShortCircuit::Or)?;
        }
        Ok(EmitVal::Int(acc))
    }

    /// Lower `acc OP rhs` as a control-flow short-circuit and phi.
    fn short_circuit(
        &mut self,
        acc: IntValue<'ctx>,
        rhs_expr: &Expr,
        kind: ShortCircuit,
    ) -> Result<IntValue<'ctx>, JitError> {
        let bool_t = self.context.bool_type();
        let eval_bb = self.context.append_basic_block(self.function, kind.eval_label());
        let skip_bb = self.context.append_basic_block(self.function, kind.skip_label());
        let merge_bb = self.context.append_basic_block(self.function, "sccont");

        let (then_target, else_target) = match kind {
            ShortCircuit::And => (eval_bb, skip_bb),
            ShortCircuit::Or => (skip_bb, eval_bb),
        };
        self.builder
            .build_conditional_branch(acc, then_target, else_target)
            .map_err(|e| format!("LLVM build_conditional_branch failed: {}", e))?;

        self.builder.position_at_end(eval_bb);
        let rhs_v = self.emit(rhs_expr)?;
        let rhs = self.expect_bool(&rhs_v, kind.name())?;
        let eval_end = self.builder.get_insert_block().expect("eval has insert block");
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("LLVM build_unconditional_branch failed: {}", e))?;

        self.builder.position_at_end(skip_bb);
        let short_v = match kind {
            ShortCircuit::And => bool_t.const_int(0, false),
            ShortCircuit::Or => bool_t.const_int(1, false),
        };
        let skip_end = self.builder.get_insert_block().expect("skip has insert block");
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(|e| format!("LLVM build_unconditional_branch failed: {}", e))?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(bool_t, "sctmp")
            .map_err(|e| format!("LLVM build_phi failed: {}", e))?;
        phi.add_incoming(&[(&rhs, eval_end), (&short_v, skip_end)]);
        Ok(phi.as_basic_value().into_int_value())
    }

    /// `(not b)` → `b XOR 1`.
    fn gen_not(&mut self, args: &[Expr]) -> Result<EmitVal<'ctx>, JitError> {
        if args.len() != 1 {
            return Err(format!(
                "`not` requires exactly 1 argument, got {}",
                args.len()
            ));
        }
        let v_e = self.emit(&args[0])?;
        let v = self.expect_bool(&v_e, "not")?;
        let one = self.context.bool_type().const_int(1, false);
        let r = self
            .builder
            .build_xor(v, one, "nottmp")
            .map_err(|e| format!("LLVM build_xor failed: {}", e))?;
        Ok(EmitVal::Int(r))
    }

    fn expect_int(&self, v: &EmitVal<'ctx>, op: &str) -> Result<IntValue<'ctx>, JitError> {
        match v {
            EmitVal::Int(iv) if iv.get_type().get_bit_width() != 1 => Ok(*iv),
            other => Err(format!(
                "operator `{}` requires integer operands, got {}",
                op,
                other.type_name()
            )),
        }
    }

    fn expect_float(&self, v: &EmitVal<'ctx>, op: &str) -> Result<FloatValue<'ctx>, JitError> {
        match v {
            EmitVal::Float(fv) => Ok(*fv),
            other => Err(format!(
                "operator `{}` requires float operands, got {}",
                op,
                other.type_name()
            )),
        }
    }

    fn expect_bool(&self, v: &EmitVal<'ctx>, op: &str) -> Result<IntValue<'ctx>, JitError> {
        match v {
            EmitVal::Int(iv) if iv.get_type().get_bit_width() == 1 => Ok(*iv),
            other => Err(format!(
                "operator `{}` operand must be bool, got {}",
                op,
                other.type_name()
            )),
        }
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

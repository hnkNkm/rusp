//! Ahead-of-time compilation: emit LLVM IR (`.ll`) or a native object
//! file (`.o`) for a Rusp source file.
//!
//! The AOT pipeline reuses `emit_defn` from `jit.rs`. The input is a
//! slice of fully-checked `Expr`s — currently restricted to a sequence
//! of `defn`s, the last of which must be `(defn main [] -> i32 ...)`.
//! The `main` defn becomes the C-ABI entry point of the produced
//! object, so it can be linked with `cc` to make an executable.
//!
//! There is no support for top-level expressions in AOT mode — they
//! have nowhere to live without a thunk, and rather than inventing
//! one we keep the surface simple: write `(defn main [] -> i32 ...)`.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::values::FunctionValue;

use crate::ast::Expr;

use super::jit::{JitError, emit_defn};

/// Emit LLVM IR (textual `.ll`) for the program. The program must be a
/// sequence of `defn`s ending with `(defn main [] -> i32 ...)`.
/// Returns the IR as a string; the caller decides where to write it.
pub fn compile_to_ll(forms: &[Expr]) -> Result<String, JitError> {
    let context = Context::create();
    let ir = build_module_ir(&context, forms)?;
    Ok(ir)
}

/// Emit a native object file at `out_path` for the program. Same input
/// shape as `compile_to_ll`. Uses the host triple and the default
/// reloc/code models, which is good enough for `cc out.o -o out`.
pub fn compile_to_obj(forms: &[Expr], out_path: &Path) -> Result<(), JitError> {
    let context = Context::create();
    let module = build_module(&context, forms)?;

    // Initialize the native target backend. Cheap if already done.
    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| format!("failed to initialize native target: {}", e))?;
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple)
        .map_err(|e| format!("failed to look up target {}: {}", triple, e))?;
    let cpu = TargetMachine::get_host_cpu_name();
    let features = TargetMachine::get_host_cpu_features();
    let machine = target
        .create_target_machine(
            &triple,
            cpu.to_str().unwrap_or("generic"),
            features.to_str().unwrap_or(""),
            OptimizationLevel::Default,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or_else(|| format!("failed to create target machine for {}", triple))?;

    machine
        .write_to_file(&module, FileType::Object, out_path)
        .map_err(|e| format!("failed to write object file: {}", e))?;
    Ok(())
}

/// Shared core: build the LLVM module from a slice of `defn` forms.
/// Validates that the last form is `(defn main [] -> i32 ...)` and
/// that no leading form is anything other than a `defn`.
fn build_module<'ctx>(
    context: &'ctx Context,
    forms: &[Expr],
) -> Result<inkwell::module::Module<'ctx>, JitError> {
    if forms.is_empty() {
        return Err("--emit: empty program (need at least `(defn main ...)`)".to_string());
    }
    for (i, f) in forms.iter().enumerate() {
        if !matches!(f, Expr::Defn { .. }) {
            return Err(format!(
                "--emit: form {} is not a `defn`; AOT mode only supports a \
                 sequence of `defn`s ending with `(defn main [] -> i32 ...)`",
                i
            ));
        }
    }
    let last = forms.last().expect("non-empty by guard above");
    let Expr::Defn { name, params, return_type, .. } = last else {
        unreachable!("matches above guarantee Defn")
    };
    if name != "main" {
        return Err(format!(
            "--emit: program's last `defn` must be named `main`, got `{}`",
            name
        ));
    }
    if !params.is_empty() {
        return Err("--emit: `main` must take zero parameters".to_string());
    }
    if !matches!(return_type, crate::ast::Type::I32) {
        return Err("--emit: `main` must return `i32`".to_string());
    }

    let module = context.create_module("rusp_aot");
    let builder = context.create_builder();
    let mut functions: HashMap<String, FunctionValue<'_>> = HashMap::new();
    let lambda_counter: Cell<u32> = Cell::new(0);
    for f in forms {
        emit_defn(context, &module, &builder, &mut functions, &lambda_counter, f)?;
    }
    Ok(module)
}

/// `build_module` + render to textual IR.
fn build_module_ir(context: &Context, forms: &[Expr]) -> Result<String, JitError> {
    let module = build_module(context, forms)?;
    Ok(module.print_to_string().to_string())
}

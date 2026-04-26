//! LLVM codegen backend (MVP).
//!
//! Submodules are filled in incrementally. The current scope (Step 2)
//! supports i32 literals and i32 arithmetic via JIT.

pub mod jit;

use inkwell::context::Context;

/// Build an empty LLVM module and return its textual IR.
///
/// Used in Step 1 as a smoke test that LLVM linkage is set up correctly.
/// Kept around because `cargo test smoke_module` is a fast diagnostic
/// when LLVM linking starts misbehaving.
pub fn smoke_module() -> String {
    let context = Context::create();
    let module = context.create_module("rusp_smoke");
    module.print_to_string().to_string()
}

pub use jit::{
    jit_eval_bool, jit_eval_bool_program, jit_eval_f64, jit_eval_f64_program, jit_eval_i32,
    jit_eval_i32_program, jit_eval_i64, jit_eval_i64_program, JitError,
};

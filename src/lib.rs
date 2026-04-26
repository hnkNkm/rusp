//! Rusp library crate.
//!
//! Exposing parser/types/eval lets the LLVM codegen module — and tests for
//! it — share the same front end as the binary REPL.

pub mod ast;
pub mod codegen;
pub mod env;
pub mod eval;
pub mod exhaustiveness;
pub mod parser;
pub mod types;

#[cfg(test)]
mod tests;

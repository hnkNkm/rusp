# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusp is a typed Lisp implemented in Rust (edition 2024): S-expression syntax with static type checking and inference. Currently ships a REPL; the project is pre-1.0 and evolving. See `README.md` (Japanese) for the user-facing language reference, and `docs/language-design.md` for the design spec.

Dependencies are minimal — only `nom` 7.1 for parsing. No external runtime.

## Essential Commands

- `cargo run` — start the REPL
- `cargo test` — run all tests; `cargo test [name]` for a single test; add `-- --nocapture` to see `println!` output
- `cargo clippy` / `cargo fmt` — lint and format

## Architecture

The REPL loop in `src/main.rs` runs every input through three sequential stages against persistent environments: **parse → type_check → eval**. A type error short-circuits before evaluation. Both a `TypeEnv` and a value `Environment` are kept across REPL iterations, so `let`/`defn` bindings persist.

- `src/parser/` — nom-based S-expression parser (`expr.rs`), with a separate pass for type-annotation syntax (`types.rs`).
- `src/ast.rs` — `Expr` and `Type` enums. Notable: numeric literals split into `Integer32`/`Integer64`/`Float`; `Let` has an optional `body` to encode let-in vs. top-level let; `Lambda` has optional return-type (inferred), `Defn` requires it.
- `src/types.rs` — type checker + `TypeEnv`. Inference fills in `Type::Inferred` placeholders.
- `src/eval.rs` — tree-walking evaluator. Assumes type-check has already passed.
- `src/env.rs` — runtime `Value` definitions and `Environment` with parent-chain lexical scoping. Built-in arithmetic/comparison/logic ops and `print`/`println`/`type-of` are registered here as `Value::BuiltinFunction`, not special-cased in the evaluator.

### Design points worth knowing before editing

- **Type env and value env are separate structures** but mirror the same scoping discipline. When you add a binding form, update both (see how `Let`/`Defn` are handled in `types.rs` and `eval.rs`).
- **Integer vs. float operators are distinct tokens** (`+` vs `+.`, etc.). There is no numeric coercion — this is enforced in the type checker, so evaluator code can assume operand types match.
- **Closures capture the environment by cloning the chain** (see `env.rs`). If you touch closure semantics, be aware this is a value-semantics capture, not a reference.
- **Tests live in `src/tests/`** (as a `#[cfg(test)] mod tests` inside the crate), not in the top-level `tests/` integration-test directory. `eval_tests.rs` is the largest and exercises the full parse→type→eval pipeline.

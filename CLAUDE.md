# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rusp is a typed Lisp implementation in Rust that combines Lisp's S-expression syntax with Rust's type safety and ownership model. The project is in active development and currently provides a REPL for interactive evaluation of Lisp expressions with type checking.

## Essential Commands

### Build & Run
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run` - Start the Rusp REPL
- `cargo run --release` - Run REPL in release mode

### Testing
- `cargo test` - Run all tests
- `cargo test [test_name]` - Run specific test by name
- `cargo test -- --nocapture` - Run tests with println! output visible
- `cargo test --lib` - Run library tests only

### Code Quality
- `cargo clippy` - Run Clippy linter for Rust-specific lints
- `cargo fmt` - Format code according to Rust standards
- `cargo fmt --check` - Check formatting without modifying files
- `cargo check` - Fast type-checking without producing binaries

## Architecture Overview

The codebase implements a typical interpreter pipeline:

1. **Parser** (`src/parser/`) - Uses nom parser combinators to parse S-expressions
   - `expr.rs` - Expression parsing logic
   - `types.rs` - Type annotation parsing
   - `error.rs` - Parse error definitions

2. **AST** (`src/ast.rs`) - Abstract syntax tree representation
   - Core expression types: Integer, Float, Bool, String, Symbol, List
   - Control flow: If, Let, Defn, Lambda, Call
   - Type annotations and inference support

3. **Type System** (`src/types.rs`) - Static type checking
   - Type environment management
   - Type inference and checking
   - Support for function types

4. **Evaluator** (`src/eval.rs`) - Expression evaluation
   - Environment-based evaluation
   - Closure support with captured environments
   - Built-in function support

5. **Environment** (`src/env.rs`) - Runtime value storage
   - Lexical scoping with environment chaining
   - Support for functions, closures, and built-in operations

6. **REPL** (`src/main.rs`) - Interactive evaluation loop
   - Integrated type checking and evaluation
   - Error reporting

## Key Design Decisions

- **nom for parsing**: Efficient parser combinator library for S-expression parsing
- **Environment chaining**: Supports lexical scoping and closures
- **Separate type and value environments**: Allows for static type checking before evaluation
- **Built-in functions**: Arithmetic and comparison operations are implemented as built-in functions in the environment

## Language Features (Planned)

According to `docs/language-design.md`, the language aims to support:
- S-expression syntax with Rust-like type annotations
- Ownership and borrowing semantics
- Pattern matching
- Generics and traits
- Async/await
- Macros (syntax and procedural)

## Development Workflow

When making changes:
1. Run `cargo clippy` to catch common mistakes and improve code quality
2. Run `cargo fmt` to ensure consistent formatting
3. Run `cargo test` to verify all tests pass
4. Use `cargo check` for quick type-checking during development
5. Test changes in the REPL with `cargo run`
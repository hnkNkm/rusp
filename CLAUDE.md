# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project called "rusp" using Rust edition 2024. Currently a minimal starter project with a simple "Hello, world!" main function.

## Essential Commands

### Build & Run
- `cargo build` - Build the project in debug mode
- `cargo build --release` - Build optimized release version
- `cargo run` - Build and run the project
- `cargo run --release` - Build and run release version

### Testing
- `cargo test` - Run all tests
- `cargo test [test_name]` - Run specific test by name
- `cargo test -- --nocapture` - Run tests with println! output visible

### Code Quality
- `cargo clippy` - Run Clippy linter for Rust-specific lints
- `cargo fmt` - Format code according to Rust standards
- `cargo fmt --check` - Check formatting without modifying files
- `cargo check` - Fast type-checking without producing binaries

## Project Structure

The codebase follows standard Rust/Cargo conventions:
- `src/main.rs` - Entry point containing the main() function
- `Cargo.toml` - Project manifest defining dependencies and metadata
- `target/` - Build artifacts (gitignored)

## Development Workflow

When making changes:
1. Run `cargo clippy` to catch common mistakes and improve code quality
2. Run `cargo fmt` to ensure consistent formatting
3. Run `cargo test` to verify all tests pass
4. Use `cargo check` for quick type-checking during development
---
description: Rust backend rules and conventions
globs: src-tauri/**/*.rs
---

# Rust Backend Rules

## Structure

- **Location**: `src-tauri/src/`
- **Flat Layout**: Do not create subdirectories. Keep modules flat.
- **Entry Point**: `lib.rs` contains all `#[tauri::command]` definitions.

## Style & Quality

- **Readability**: Prioritize clarity over cleverness.
- **Documentation**: All public items must have doc comments (`///`).
- **Formatting**: Must pass `cargo fmt`.
- **Linting**: Must pass `cargo clippy -- -D warnings`.
- **No Unexplained Allows**: `#[allow(...)]` must have a reason comment.

## Testing

- **Unit Tests**: Place in `#[cfg(test)] mod tests {}` within the source file.
- **Musashi Tests**: Ensure `cargo test` passes the 60 instruction tests in `src-tauri/test/`.

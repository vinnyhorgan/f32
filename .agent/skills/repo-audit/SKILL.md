---
name: repo-audit
description: Performs a deep audit of the repository for code quality, style, and architectural compliance. Use when asked to "audit", "check", or "review" the entire project.
---

# Repository Audit Skill

## When to use this skill

- When the user asks for a "health check" or "audit".
- Before a major release to ensure quality.
- When cleaning up technical debt.

## Audit Steps

### 1. Backend Audit (Rust)

Run the following commands to check the Rust backend:

```bash
cd src-tauri
# Check formatting
cargo fmt -- --check
# Check for common mistakes and best practices
cargo clippy -- -D warnings
# Run tests (must be single-threaded due to global state)
cargo test -- --test-threads=1
```

**Manual Checks:**

- [ ] Verify no subdirectories in `src-tauri/src/` (Flat structure rule).
- [ ] Check `Cargo.toml` for unused dependencies (manual review).
- [ ] Ensure all public items in `lib.rs` and modules have `///` doc comments.
- [ ] Search for `#[allow(...)]` and verify each has a `reason` comment.

### 2. Frontend Audit (TypeScript)

Run the following commands to check the frontend:

```bash
# Check for linting errors and type errors
pnpm lint
```

**Manual Checks:**

- [ ] Verify all Tauri commands are called via `EmulatorAPI` (search for direct `invoke` calls).
- [ ] Check for `any` types in `src/`.
- [ ] Ensure components are in `src/components/` or `src/components/ui/`.

### 3. Report

Generate a summary report of the findings.

- List any linting warnings or errors.
- List any architectural violations.
- List any testing failures.
- Provide recommendations for fixes.

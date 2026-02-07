---
description: Run all tests (Rust backend and TypeScript frontend)
---

# Run Tests

## Backend Tests (Rust)

1. Run cargo tests (sequential for global state safety)

```bash
cd src-tauri
cargo test -- --test-threads=1
```

## Frontend Tests (TypeScript)

2. Run linting

```bash
pnpm lint
```

---
name: new-command
description: Instructions for adding a new Tauri command. Use when the user wants to expose Rust functionality to the frontend.
---

# Add Tauri Command Skill

## When to use this skill

- When the user asks to "add a command", "expose a function", or "connect frontend to backend".

## Workflow

### 1. Backend Implementation (`src-tauri/src/lib.rs`)

1.  Define the function with `#[tauri::command]`.
2.  Use `Result<T, String>` for return types to handle errors gracefully.
3.  Add the function name to the `invoke_handler!` macro in the `run()` function.

```rust
/// Doc comment explaining the command
#[tauri::command]
fn my_new_command(arg: String) -> Result<String, String> {
    // Implementation
    Ok("success".to_string())
}
```

### 2. Frontend API Wrapper (`src/lib/emulator-api.ts`)

1.  Add a static method to the `EmulatorAPI` class.
2.  Use the `invoke` function to call the backend command.
3.  Ensure inputs and outputs are typed (do not use `any`).

```typescript
static async myNewCommand(arg: string): Promise<string> {
    return await invoke('my_new_command', { arg });
}
```

### 3. Type Definitions (`src/lib/emulator-types.ts`)

1.  If the command returns a custom struct, define the interface here.
2.  Ensure the interface matches the Rust struct (which should derive `Serialize`).

### 4. Verification

1.  Run the app: `pnpm tauri dev`
2.  Test the command from the frontend.

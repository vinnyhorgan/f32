---
name: new-component
description: Instructions for adding a new React UI component. Use when the user asks to "add a button", "create a view", or "build a component".
---

# Add UI Component Skill

## When to use this skill

- When creating new visual elements or views in the frontend.

## Workflow

### 1. Identify Type

- **Primitive**: Is it a basic building block (button, input, card)? -> Use `shadcn/ui`.
- **Feature**: Is it a specific part of the emulator (RegisterView, MemoryDump)? -> Create in `src/components/`.

### 2. Create Component

**If Primitive (shadcn/ui):**

```bash
npx shadcn@latest add [component-name]
```

**If Feature:**
Create `src/components/[ComponentName].tsx`.

- Use Functional Component syntax.
- Use `lucide-react` for icons if needed.
- Use `tailwind` classes for styling (no separate CSS files).

### 3. Implementation Guidelines

- **Props**: Define an interface for props.
- **State**: Use local `useState` for UI state, or `useEmulatorStore` (Zustand) for global emulator state.
- **Interactivity**: If it triggers a backend action, call `EmulatorAPI.actionName()`.

### 4. Export

- Export the component as default or named export.
- Import and use it in the parent component or `App.tsx`.

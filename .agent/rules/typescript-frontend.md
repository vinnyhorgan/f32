---
description: TypeScript frontend rules and conventions
globs: src/**/*.ts, src/**/*.tsx
---

# TypeScript Frontend Rules

## Structure

- **API Layer**: All backend communication MUST go through `src/lib/emulator-api.ts`.
- **Types**: Sync types in `src/lib/emulator-types.ts` with Rust structs.
- **Components**: specific feature components in `src/components/`, generic UI in `src/components/ui/`.

## Style & Quality

- **Strict TypeScript**: No `any` types allowed.
- **React**: Functional components + hooks only.
- **Styling**: Tailwind CSS v4. Use utility classes.
- **Linting**: Must pass `npm run lint`.

## Workflow

- **State**: Use Zustand for global state.
- **Async**: Handle async Tauri commands with proper loading states.

---
description: Guidelines for agent tool usage and problem solving workflow
alwaysApply: true
---

# Agent Workflow & Tool Usage

## Testing & Verification

- Use the **`tauri` MCP server** to test the running application.
- Validate application state using `driver_session`, `webview_dom_snapshot`, and other Tauri driver tools.
- Verification should involve actual interaction with the app when possible.

## Documentation & Research

- Always pull the **latest documentation** using the **`context7` MCP server** (`query-docs`) before implementing new features or using unfamiliar libraries.
- Don't assume API knowledge for rapidly evolving libraries; check the docs first.

## Problem Solving

- Use the **`sequential-thinking` tool** to break down **complex problems** into manageable steps.
- Before jumping into implementation on difficult tasks, outline your logic and potential pitfalls using sequential thinking.
- Revisit your thinking process if you encounter unexpected roadblocks.

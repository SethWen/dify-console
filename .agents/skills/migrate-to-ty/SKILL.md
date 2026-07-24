---
name: migrate-to-ty
description: Migrate Python static type checking from Pyright to Ty, map configuration files, configure VSCode development environment, and resolve common Ty-specific type checker errors. Use when user wants to migrate type checkers to Ty, configure Ty, or fix Ty type checker errors.
---

# Migrate to Ty

This skill guides the agent in migrating a Python project's static type checker from `pyright` to `ty` (Astral's fast Rust-based type checker), setting up the VSCode environment, and resolving typical `ty` type checking errors.

## Quick Start

1. Install `ty` in dev dependency group:
   ```bash
   uv add --group dev ty
   ```
2. Replace `[tool.pyright]` in `pyproject.toml` with `[tool.ty]`:
   ```toml
   [tool.ty.src]
   include = ["src/**/*.py"]
   exclude = ["src/demo/**", "src/typings/**"]

   [tool.ty.environment]
   extra-paths = ["src", "src/typings"]

   [tool.ty.rules]
   unused-type-ignore-comment = "ignore"
   ```

## Workflows

### Phase 1: Dependency & Configuration Migration
1. Remove `pyright` from `pyproject.toml` dependency groups and run `uv sync`.
2. Convert include/exclude paths and search paths into `[tool.ty]` syntax.
3. Initially set all strict checking rules to `"ignore"` in `[tool.ty.rules]` to ensure a smooth transition.

### Phase 2: VSCode LSP Alignment
1. Modify `.vscode/settings.json` to disable Pylance type diagnostics:
   ```json
   "python.analysis.typeCheckingMode": "off"
   ```
2. Recommend the official `astral-sh.ty-vscode` extension in `.vscode/extensions.json`.

### Phase 3: Incremental Error Resolution
1. Change a specific rule from `"ignore"` to `"error"` in `pyproject.toml`.
2. Run `uv run ty check` to identify errors.
3. Apply standard fixing patterns (Type Narrowing, TypedDict, or `ty:ignore` comments).
4. See [REFERENCE.md](REFERENCE.md) for concrete fixing code patterns.

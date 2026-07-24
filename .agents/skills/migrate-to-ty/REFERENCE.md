# Reference Guide: Resolving ty Type Checker Warnings and Errors

This reference guide provides mapping tables and concrete fixing templates for the most common errors and warnings encountered when migrating a Python project to `ty`.

---

## 1. Configuration Mapping Reference

| Pyright Option | Ty (tool.ty) Equivalent | Description |
| :--- | :--- | :--- |
| `include` | `[tool.ty.src] include` | List of glob patterns to check. |
| `exclude` | `[tool.ty.src] exclude` | Glob patterns to ignore from type checking. |
| `extraPaths` | `[tool.ty.environment] extra-paths` | Additional module search directories. |
| `stubPath` | `[tool.ty.environment] extra-paths` | Path to custom stub files (merged into search paths in `ty`). |
| `typeCheckingMode` | `[tool.ty.rules]` | Custom rules levels (e.g., `invalid-assignment = "error"`). |

---

## 2. Inline Ignore Rules
* `ty` **does not support** standard bracketed comments like `# type: ignore[rule-name]` (it will treat them as invalid and ignore them).
* **Supported syntax**:
  1. `# ty:ignore[rule-name]` (Recommended for precise tool-specific ignore).
  2. `# type: ignore` (Generic ignore for all type checkers, no brackets).

---

## 3. Common Error Fixing Templates

### Pattern A: `not-iterable` (Type Widening)
* **Problem**: Dict values containing different types are inferred as a large Union (e.g., `list | str | Model`). Iterating over keys of such a dict triggers `not-iterable` since `str` or `Model` are not iterable.
* **Fix**: Use `TypedDict` to enforce key-specific types and prevent automatic type widening.
* **Example**:
  ```python
  # Before (triggers not-iterable on info["ids"])
  groups = {
      "KEY": {"ids": [], "model": MyModel}
  }
  for info in groups.values():
      db_ids = [item[0] for item in info["ids"]]
  
  # After (Fixed)
  from typing import TypedDict
  class GroupInfo(TypedDict):
      ids: list[tuple[int, str]]
      model: type
  
  groups: dict[str, GroupInfo] = {
      "KEY": {"ids": [], "model": MyModel}
  }
  ```

### Pattern B: `invalid-assignment` (Variable Re-use)
* **Problem**: A local variable name is bound to a type (e.g., via local imports), and is later re-used in the same function scope with an incompatible type (e.g., as a loop variable).
* **Fix**: Rename the loop variable or the imported variable to avoid name clash.
* **Example**:
  ```python
  # Before (invalid-assignment on cfg)
  from pkg.config import config as cfg
  for cfg in configs:  # Error: dict is not assignable to Config
      pass
  
  # After (Fixed)
  from pkg.config import config as cfg
  for config_item in configs:
      pass
  ```

### Pattern C: `no-matching-overload` (Decorators on Protocols)
* **Problem**: Complex Python framework decorators (e.g., Temporal workflow updates) that wrap instance methods cause overload resolution to fail. The checker complains that argument list `[SelfType, ParamType]` is not assignable to `ParamSpec`.
* **Fix**: This is an issue with third-party stub definitions. Use `# ty:ignore[no-matching-overload]` on the calling line to suppress it.
* **Example**:
  ```python
  # Fixed using inline ignore comment
  result = await handle.execute_update(  # ty:ignore[no-matching-overload]
      WorkflowClass.method_name,
      Argument(val=val)
  )
  ```

### Pattern D: `unresolved-attribute` (None Type & Base Classes)
* **Problem 1**: A property is accessed after an asynchronous wait condition, but the checker still infers the property as `None | Value`.
* **Fix 1**: Use `assert` to narrow the type.
  ```python
  # Fixed via type narrowing assert
  await wait_condition(lambda: self.value is not None)
  assert self.value is not None
  val = self.value.property
  ```
* **Problem 2**: Accessing child class attributes on an object inferred as a base class.
* **Fix 2**: Typecast or annotate the query variable as `Any`.
  ```python
  # Fixed via Any annotation
  child: Any = parent.get_item()
  width = child.Width
  ```

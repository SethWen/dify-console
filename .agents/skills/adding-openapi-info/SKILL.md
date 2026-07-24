---
name: adding-openapi-info
description: Use when adding or updating API endpoints and wanting to include OpenAPI documentation with Chinese tags, summary, and description
---

# Adding OpenAPI Info

## Overview

为 Litestar API 端点添加 OpenAPI 文档信息，包括中文 tags、summary 和 description。

## When to Use

- 创建新的 API 端点
- 更新现有端点需要补充文档
- 参照已有端点模式添加类似信息

## Core Pattern

### Controller 级别

```python
# Before
class AuthController(Controller):
    tags = ["Auth"]
    path = "/auth"

# After
class AuthController(Controller):
    tags = ["认证管理"]
    path = "/auth"
```

### Endpoint 级别

```python
# Before
@post("/login", status_code=status_codes.HTTP_200_OK)
async def login(...):

# After
@post(
    "/login",
    status_code=status_codes.HTTP_200_OK,
    summary="用户登录",
    description="使用用户名和密码登录，登录成功后设置会话cookie",
)
async def login(...):
```

## Quick Reference

| 字段 | 用途 | 示例 |
|------|------|------|
| `tags` | API 分组（中文） | `tags = ["认证管理"]` |
| `summary` | 端点简短描述 | `summary="用户登录"` |
| `description` | 详细说明 | `description="使用用户名和密码登录，登录成功后设置会话cookie"` |

## Common Mistakes

- **忘记添加**: 每个 endpoint 都应有 summary
- **summary 太长**: 应简洁，控制在 20 字以内
- **description 与 summary 重复**: description 应提供更多细节
- **tags 使用英文**: 应使用中文分组名称

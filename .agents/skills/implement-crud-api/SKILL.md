---
name: implement-crud-api
description: Use when implementing CRUD APIs for a database table - creating new endpoints, repositories, and models following OpenSpec workflow
---

# Implement CRUD API

为数据库表实现 CRUD 接口的标准化流程。

## 核心流程

```
proposal → design → specs → tasks → implement → update spec → archive
```

## 1. 创建 OpenSpec 变更

```bash
openspec new change "<name>-crud-api"
```

命名规范：`/<entity-name>-crud-api`

## 2. 创建 Artifacts

### 2.1 Proposal (what & why)

```markdown
## Why
[解释动机，这个功能解决什么问题]

## What Changes
- **POST** `/<entity>/create` - 批量新增
- **POST** `/<entity>/delete` - 批量删除
- **POST** `/<entity>/list` - 分页查询

## Capabilities
- `<entity-management>`: [描述能力]
```

### 2.2 Design (how)

```markdown
## Context
- 现有模型: `src/pkg/models/db_schemas.py`
- 现有模式: SQLAlchemyAsyncRepository
- 响应格式: `BasicResponse<T>`

## Goals / Non-Goals
**Goals:**
- 实现批量新增
- 实现批量删除
- 实现分页查询

**Non-Goals:**
- 不实现单个删除（批量接口）

## Decisions
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/<entity>/create` | 批量新增 |
| POST | `/<entity>/delete` | 批量删除 |
| POST | `/<entity>/list` | 分页查询 |
```

### 2.3 Specs (requirements + scenarios)

```markdown
## ADDED Requirements

### Requirement: Batch create [entity]
系统 SHALL 支持...

#### Scenario: Successfully create batch
- **WHEN** 用户提交...
- **THEN** 系统...

### Requirement: Batch delete [entity]
...

### Requirement: Paginated search [entity]
...
```

## 3. 实现代码

### 3.1 创建 Pydantic 模型 (`src/pkg/api/<entity>_model.py`)

```python
from pydantic import BaseModel, Field, RootModel
from typing import List

class [Entity]CreateItem(BaseModel):
    name: str = Field(..., description="名称")
    # ... 其他字段

class [Entity]BatchCreateRequest(RootModel[List[[Entity]CreateItem]]):
    """直接接收数组"""
    def __iter__(self):
        return iter(self.root)
    def __len__(self) -> int:
        return len(self.root)

class [Entity]ListRequest(BaseModel):
    name: str | None = None
    type: str | None = None
    page: int = Field(default=1, ge=1)
    size: int = Field(default=10, ge=1, le=100)

class [Entity]Read(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: int
    # ... 其他字段

class Paginated[Entity]Response(BaseModel):
    list: List[[Entity]Read]
    total: int
    page: int
    size: int
```

### 3.2 创建 Repository (`src/pkg/repositories/<entity>.py`)

```python
from advanced_alchemy.repository import SQLAlchemyAsyncRepository
from sqlalchemy import func, select

from pkg.models.db_schemas import [Entity]

class [Entity]Repository(SQLAlchemyAsyncRepository[[Entity]]):
    model_type = [Entity]

    async def create_batch(self, items: list[[Entity]]) -> list[[Entity]]:
        created = []
        for item in items:
            created.append(await self.add(item))
        await self.session.commit()
        return created

    async def delete_batch(self, ids: list[int]) -> int:
        if not ids:
            return 0
        stmt = select([Entity]).where([Entity].id.in_(ids))
        result = await self.list(statement=stmt)
        for item in result:
            await self.delete(item.id)
        await self.session.commit()
        return len(result)

    async def paginate_search(
        self, name: str | None, entity_type: str | None, page: int, size: int
    ) -> tuple[list[[Entity]], int]:
        stmt = select([Entity])
        if name:
            stmt = stmt.where([Entity].name.contains(name))
        if entity_type:
            stmt = stmt.where([Entity].type == entity_type)

        count_stmt = select(func.count()).select_from(stmt.subquery())
        total = (await self.session.execute(count_stmt)).scalar() or 0

        offset = (page - 1) * size
        stmt = stmt.limit(size).offset(offset)
        result = await self.session.execute(stmt)
        return list(result.scalars().all()), total
```

### 3.3 添加 Provider (`src/pkg/api/providers.py`)

```python
from pkg.repositories.<entity> import [Entity]Repository

async def provide_<entity>_repo(db_session: AsyncSession) -> [Entity]Repository:
    return [Entity]Repository(session=db_session)
```

### 3.4 创建 Controller (`src/pkg/api/<entity>_controller.py`)

```python
from litestar import Controller, post, status_codes
from litestar.di import Provide
from litestar.params import Body

from pkg.api.model import BasicResponse
from pkg.api.<entity>_model import (
    [Entity]BatchCreateRequest,
    [Entity]BatchDeleteRequest,
    [Entity]ListRequest,
    [Entity]Read,
    Paginated[Entity]Response,
)
from pkg.api.providers import provide_<entity>_repo

class [Entity]Controller(Controller):
    tags = ["[Entity]"]
    path = "/<entity>"
    dependencies = {"<entity>_repo": Provide(provide_<entity>_repo)}

    @post("/create", status_code=status_codes.HTTP_200_OK)
    async def create(self, data: [Entity]BatchCreateRequest, <entity>_repo) -> BasicResponse[List[[Entity]Read]]:
        items = [[Entity](**item.model_dump()) for item in data]
        created = await <entity>_repo.create_batch(items)
        return BasicResponse(data=[[Entity]Read.model_validate(c) for c in created])

    @post("/delete", status_code=status_codes.HTTP_200_OK)
    async def delete(self, data: [Entity]BatchDeleteRequest = Body(...), <entity>_repo = Body(default=...)) -> BasicResponse[int]:
        count = await <entity>_repo.delete_batch(data.ids)
        return BasicResponse(data=count)

    @post("/list")
    async def list(self, data: [Entity]ListRequest, <entity>_repo) -> Paginated[Entity]Response:
        items, total = await <entity>_repo.paginate_search(
            name=data.name, entity_type=data.type, page=data.page, size=data.size
        )
        return Paginated[Entity]Response(
            list=[[Entity]Read.model_validate(i) for i in items],
            total=total, page=data.page, size=data.size
        )
```

### 3.5 注册 Controller (`src/main_api.py`)

```python
from pkg.api.<entity>_controller import [Entity]Controller

app = Litestar(route_handlers=[..., [Entity]Controller], ...)
```

## 4. 验证

```bash
uv run ruff check src/pkg/api/<entity>_model.py src/pkg/api/<entity>_controller.py src/pkg/repositories/<entity>.py
uv run pyright src/pkg/api/<entity>_model.py src/pkg/api/<entity>_controller.py src/pkg/repositories/<entity>.py
```

## 5. 更新 Spec 为最终实现

根据用户调整后的代码更新 spec，确保与实现一致。

## 6. 归档

```bash
openspec archive <name>-crud-api
```

## 快速参考

| 操作 | 命令 |
|------|------|
| 创建变更 | `openspec new change "<name>"` |
| 查看状态 | `openspec status --change "<name>"` |
| 创建 artifacts | 依次创建 proposal, design, specs, tasks |
| 归档 | `openspec archive <name>` |

## 常见问题

**Q: 删除接口返回什么？**  
A: `BasicResponse<int>` 返回删除数量

**Q: 分页参数名称？**  
A: 使用 `page` 和 `size`（不是 `page_size`）

**Q: 如何处理空数组？**  
A: Pydantic 的 `min_length=1` 验证会自动拒绝空数组

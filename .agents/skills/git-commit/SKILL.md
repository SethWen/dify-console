---
name: git-commit
description: 生成 commit message 并提交已 staged 的改动
allowed-tools: Bash(git add:*), Bash(git status:*), Bash(git diff:*), Bash(git log:*), Bash(git commit:*)
---

# Git Commit

为已 staged 的改动生成 commit message 并提交。

## 执行步骤

### 1. 检查 git 状态和 staged 改动

执行以下命令查看当前状态：

```bash
git status
git diff --cached
git log --oneline -5
```

### 2. 分析改动内容

根据 git diff --cached 的输出，分析以下内容：
- 改动的类型 (新增功能 / bug 修复 / 重构 / 文档更新 / 测试 等)
- 改动的文件数量和范围
- 改动的主要目的和影响

### 3. 生成 Commit Message

根据改动内容生成符合项目规范的 commit message：
- 使用中文描述 (项目规范)
- 格式：`{类型}: {简短描述}`
- 类型可选：feat, fix, refactor, docs, test, chore, perf, ci
- 如果是多个独立改动，使用多个 commit

### 4. 创建 Commit

执行 git commit 命令：

```bash
git commit -m "{commit message}"
```

添加 co-author:

```bash
git commit -m "{commit message}" -m "Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

### 5. 验证

提交完成后执行：

```bash
git status
```

## 注意事项

- 只处理已 staged (git add) 的改动
- 如果有 unstaged 的改动，需要先询问用户是否要一起提交
- 确保改动不包含敏感信息 (.env, credentials 等)
- 遵循项目 commit 规范

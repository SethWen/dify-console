# Dify Console CLI Tool (Rust 实现)

这是一个使用 Rust 编写的命令行工具，用于与 Dify WebUI (Console) 内部 API 进行交互。它支持导入（Import）和导出（Export）应用 DSL（Workflows/Chatflows），并在导入时支持模型等配置的原位替换与发布，同时提供多环境 App ID 映射管理与 Session 缓存。

## 功能特性

- **模块化架构**：项目结构清晰拆分为 CLI 定义、核心命令（导入/导出）、API 客户端、Session 缓存管理及辅助工具等模块。
- **彩色终端输出**：集成 `clap` 的新版终端渲染特性与自定义 ANSI 样式主题，在 TTY 终端中呈现高可读性的彩色帮助与错误提示。
- **纯 Rust 依赖**：禁用 `native-tls` 并开启 `rustls-tls`，无 native C 语言库依赖，在 Windows、macOS 和 Linux 上均可流畅进行交叉编译。
- **智能 Session 缓存**：以 `host + 邮箱` 为 Key，将登录 Cookie 和 CSRF Token 缓存在系统标准配置目录中。执行操作前自动检查会话有效性，免去频繁输入密码。
- **交互式密码引导**：若未指定密码且缓存失效，工具会通过 `rpassword` 安全地在终端引导输入隐式密码。
- **多环境结构化映射管理 (`app_mapping.json`)**：
  - **结构化定义**：通过强类型 `struct` 精确区隔每个环境（如 `dev`、`preview`、`production`）下的应用文件映射与参数替换规则。
  - **严格读取校验**：在读取 mapping 文件时发生任何格式或读取错误时会立刻报错熔断，决不以空映射进行静默覆盖，保障历史映射数据的完整性。
  - **导入写保护**：在执行 `import` 时实施写保护。只有当指定的 mapping 关系文件**原本不存在**时才会在最后写入新关系；若文件已存在，则保持只读，保护 CI/CD 流程中的配置文件干净。
  - **白名单跳过机制**：批量导入时，如果发现扫描到的本地 YAML 在当前指定的环境中没有映射的 App ID，则会**直接跳过该应用的导入与发布**，防止在预览/正式环境因为误扫而自动创建新的垃圾应用。
- **YAML 路径字段原位替换 (Replace Rules)**：
  - 导入前可根据当前环境下的 `replace_rules` 对 YAML DSL 文本进行高精度的原位条件修改（如替换不同 Dify 实例中的大模型提供商 `provider` 和名称 `name`）。
  - 支持 `path_segments:target_key` 格式，定位明确。且当路径层级中包含 `Array`（如 `nodes`）时，会自动对数组内的每一个成员递归向下匹配。
  - 支持 `from` 为 `*` 的通配修改，且支持自动保留 Boolean / Number / String 的原生 JSON 类型。
- **DSL 导入后自动发布**：支持 `-P, --publish` 选项。在应用（只限 `workflow` 和 `advanced-chat`）导入成功后，全自动请求 Dify 接口完成部署上线。

## 目录结构

```
src/
├── main.rs            # CLI 程序入口
├── cli.rs             # 命令行参数及彩色样式定义 (Cli, Commands)
├── utils.rs           # 强类型配置定义与通用辅助工具 (ReplaceRule, EnvConfig, parse_dsl_metadata, replace_yaml_content)
├── dify_api.rs        # Dify Console 接口客户端与身份认证 (DifyClient)
└── session.rs         # 跨平台 Session 缓存加载与存储
```

## 编译方法

在项目根目录下使用 Rust 构建工具 `cargo` 进行构建：

```bash
# 检查代码编译
cargo check

# 编译开发版本
cargo build

# 编译生产模式发布版本 (二进制文件位于 target/release/dify-console)
cargo build --release
```

---

## 配置文件规范 (`app_mapping.json`)

重构后的配置文件采用结构化的嵌套设计，示例如下：

```json
{
  "preview": {
    "apps": {
      "workflows/1招标文件解析.yml": "46294951-f14e-4a14-84a5-b74929411459"
    },
    "replace_rules": []
  },
  "dev": {
    "apps": {
      "workflows/1招标文件解析.yml": "461b6613-05e8-4239-90a4-e0ca1fd00f15",
      "workflows/2投标文件解析.yml": "399121e5-ec22-4012-a4d7-b467eb8c6494"
    },
    "replace_rules": [
      {
        "path": "workflow.graph.nodes.data.model:provider",
        "from": "langgenius/deepseek/deepseek",
        "to": "openai"
      },
      {
        "path": "workflow.graph.nodes.data.model:name",
        "from": "deepseek-chat",
        "to": "gpt-4o"
      }
    ]
  }
}
```

* **`apps`**：记录文件相对路径到 Dify 实例中 App ID 的映射。
* **`replace_rules`**：导入该环境时需要执行的条件替换规则。
  * `path`：冒号前为点分割的 JSON 路径（如 `workflow.graph.nodes.data.model`），冒号后为要修改的键名（如 `provider`）。
  * `from`：如果字段当前值匹配此内容（或指定为 `*` 通配符），则执行替换。
  * `to`：替换后的目标值。

---

## 使用说明

### 1. 导出 DSL (Export)

将 Dify 平台上的应用导出为本地 DSL (YML 格式) 配置文件。

```bash
# 基础导出：使用默认 Tag 过滤应用，若缓存失效将交互式提示输入密码
./target/release/dify-console export \
  --url http://localhost:8080 \
  --email admin@example.com

# 完整参数导出：指定密码、自定义输出目录、导出所有类型的应用、自定义 Tag 及指定映射环境
./target/release/dify-console export \
  -u http://localhost:8080 \
  -e admin@example.com \
  -p my_secret_password \
  -o ./difydsl \
  -m all \
  -t "我的标签" \
  -E dev \
  --map-file ./difydsl/app_mapping.json
```

**导出常用参数说明：**
* `-u, --url` (必填): Dify 控制台的 Base URL。
* `-e, --email` (必填): 登录邮箱账号。
* `-p, --password` (可选): 登录密码。
* `-o, --output` (可选): 本地保存 DSL 的根目录，默认为 `difydsl`。
* `-m, --modes` (可选): 导出的应用模式（如 `workflow,advanced-chat` 或 `all`）。
* `-t, --tag` (可选): 过滤指定标签的应用。
* `-E, --env` (可选): 写入映射文件的目标环境名，默认为 `dev`。
* `--map-file` (可选): 映射关系 JSON 文件的保存路径，默认为 `difydsl/app_mapping.json`。

---

### 2. 导入/更新 DSL (Import)

支持将单个本地 YML 配置文件导入到 Dify，或对整个目录进行批量导入。

#### 选项 A：导入单个 DSL 文件
```bash
# 单个文件导入并自动发布应用 (若指定为 dev, 也会根据 dev 下的 replace_rules 运行替换)
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -f ./difydsl/workflows/my_workflow.yml \
  -E dev \
  --publish
```

#### 选项 B：批量导入整个目录 (自动应用写保护和无映射跳过)
```bash
# 批量导入目录下的 DSL，自动根据映射文件在目标环境就地覆盖更新
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -d ./difydsl \
  -m ./difydsl/app_mapping.json \
  -E dev \
  -P
```

**导入常用参数说明：**
* `-f, --file` (与 `-d` 二选一): 导入的单个 DSL (YML) 文件的路径。
* `-d, --dir` (与 `-f` 二选一): 批量导入的包含 DSL 文件的文件夹路径。
* `-a, --app-id` (仅适用于 `-f`): 指定覆盖更新的 Dify 目标 App ID。
* `-m, --map-file` (仅适用于 `-d`): 映射关系文件的路径，默认是 `difydsl/app_mapping.json`。
* `-E, --env` (必填): 当前操作的目标环境（如 `dev`、`preview`、`production`）。
* `-P, --publish` (可选): 导入成功后自动发布（仅适用于工作流/Chatflow）。

# Dify Console CLI Tool (Rust 实现)

这是一个使用 Rust 编写的命令行工具，用于与 Dify WebUI (Console) 内部 API 进行交互。它支持导入（Import）和导出（Export）应用 DSL（Workflows/Chatflows），管理多环境的 App ID 映射关系，并在本地进行会话缓存与校验。

## 功能特性

- **模块化架构**：项目结构清晰拆分为 CLI 定义、核心命令（导入/导出）、API 客户端、Session 缓存管理及辅助工具等模块，具有良好的可维护性。
- **彩色终端输出**：集成 `clap` 的新版终端渲染特性与自定义 ANSI 样式主题，在 TTY 终端中呈现高可读性的彩色帮助与错误提示。
- **纯 Rust 依赖**：禁用 `native-tls` 并开启 `rustls-tls`，无 native C 语言库依赖，在 Windows、macOS 和 Linux 上均可极其顺滑地进行交叉编译。
- **智能 Session 缓存**：以 `host + 邮箱` 为 Key，将登录 Cookie 和 CSRF Token 缓存在系统标准配置目录中（例如 Linux 下的 `~/.config/dify-cli/sessions.json`）。执行操作前自动检查会话有效性，免去频繁输入密码的烦恼。
- **交互式密码引导**：若未通过 `-p/--password` 参数指定密码且 Session 缓存失效，工具会通过 `rpassword` 安全地在终端引导输入隐式密码。
- **多环境映射管理**：通过 `app_mapping.json` 自动记录并追踪应用在多套环境（如 `dev`、`preview`、`production`）中的 App ID，导入时自动就地更新，无需手动维护 ID 映射。

## 目录结构

```
src/
├── main.rs            # CLI 程序入口
├── cli.rs             # 命令行参数及彩色样式定义 (Cli, Commands)
├── utils.rs           # 通用辅助工具 (sanitize_filename, get_mode_folder)
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

### 交叉编译示例

得益于纯 Rust 实现的 HTTP 和 TLS，在非目标平台（如在 Linux 上编译 Windows 格式的发布版本）上可轻松进行交叉编译：

```bash
# 交叉编译至 Windows (64位)
cargo build --release --target x86_64-pc-windows-gnu

# 交叉编译至 macOS (使用 cross 工具)
cross build --release --target x86_64-apple-darwin
```

## 使用说明

编译成功后，可直接在终端中调用编译出的二进制文件。运行 `-h/--help` 查看彩色帮助菜单：

```bash
./target/release/dify-console --help
```

### 1. 导出 DSL (Export)

将 Dify 平台上的应用导出为本地 DSL (YML 格式) 配置文件。

```bash
# 基础导出：使用默认 Tag ('智能投标') 过滤应用，若缓存失效将交互式提示输入密码
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
* `-p, --password` (可选): 登录密码，省略则会尝试读取缓存或在终端交互式提示输入。
* `-o, --output` (可选): 本地保存 DSL 的根目录，默认为 `difydsl`。
* `-m, --modes` (可选): 导出的应用模式，多个以英文逗号分隔（如 `workflow,advanced-chat`），或指定 `all`，默认导出 `workflow,advanced-chat`。
* `-t, --tag` (可选): 过滤指定标签的应用。
* `-E, --env` (可选): 写入映射文件的目标环境名，默认为 `dev`。
* `--map-file` (可选): 映射关系 JSON 文件的保存路径，默认为 `difydsl/app_mapping.json`。

---

### 2. 导入/更新 DSL (Import)

支持将单个本地 YML 配置文件导入到 Dify，或对整个目录进行批量导入（根据映射文件自动判断创建新应用或就地覆盖更新）。

#### 选项 A：导入单个 DSL 文件
```bash
# 单个文件导入（创建一个新应用）
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -f ./difydsl/workflows/my_workflow.yml \
  -E dev

# 单个文件就地更新（指定已有 App ID 进行覆盖更新）
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -f ./difydsl/workflows/my_workflow.yml \
  -a your-existing-app-id \
  -E dev
```

#### 选项 B：批量导入整个目录
```bash
# 批量导入目录下的 DSL (忽略符号链接)，并根据映射文件自动执行覆盖更新或新建
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -d ./difydsl \
  -m ./difydsl/app_mapping.json \
  -E dev
```

**导入常用参数说明：**
* `-f, --file` (与 `-d` 二选一): 导入的单个 DSL (YML) 文件的路径。
* `-d, --dir` (与 `-f` 二选一): 批量导入的包含 DSL 文件的文件夹路径。
* `-a, --app-id` (仅适用于 `-f`): 指定覆盖更新的 Dify 目标 App ID。
* `-m, --map-file` (仅适用于 `-d`): 映射关系文件的路径，默认是 `difydsl/app_mapping.json`。
* `-E, --env` (必填): 当前操作的目标环境（如 `dev`、`preview`、`production`），导入时会依据该环境下的关系查找对应的 App ID 进行更新。

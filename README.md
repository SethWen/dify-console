# Dify Console CLI Tool (Rust 实现)

这是一个使用 Rust 编写的命令行工具，用于与 Dify WebUI (Console) 内部 API 进行交互。它支持导入（Import）和导出（Export）应用 DSL（Workflows/Chatflows），并在本地进行 Session 状态缓存与校验。

## 功能特性

- **纯 Rust 依赖**：无 native C-link 依赖（禁用 `native-tls`，开启 `rustls-tls`），支持在各种平台上极其顺滑地交叉编译（Windows, macOS, Linux）。
- **Session 缓存**：以 `host:port + 邮箱账户` 为 Key，将认证 Cookie 与 CSRF Token 缓存在系统的标准 config 目录中（如 Linux `~/.config/dify-cli/sessions.json`）。在每次操作前自动调用 Dify 接口进行会话有效性校验，避免频繁输入密码。
- **命令行参数与交互式密码**：支持通过 `--password` 参数传递密码，如未传递且缓存失效，会自动在终端引导交互式隐式输入密码，保证安全性。
- **软链接（Symlink）与去重**：导出时，主文件保存为 `{app_id}.yml`，并在同目录下创建 `clean_app_name.yml` 指向它的软链接。若重名则自动添加 `_1`, `_2` 后缀，避免冲突。

## 编译方法

在项目根目录下，使用 `cargo` 进行构建：

```bash
# 检查编译
cargo check

# 开发模式编译
cargo build

# 生产模式编译 (生成二进制文件位于 target/release/dify-console)
cargo build --release
```

### 交叉编译

因为是纯 Rust 实现的 HTTP & TLS，你可以简单使用 `cross` 或者是直接指定 target 编译：

```bash
# 例如交叉编译至 Windows (在 Linux 主机上)
cargo build --release --target x86_64-pc-windows-gnu

# 交叉编译至 macOS (使用 cross)
cross build --release --target x86_64-apple-darwin
```

## 使用说明

编译成功后，可以直接使用构建好的二进制文件。你可以先运行 `--help` 查看说明：

```bash
./target/release/dify-console --help
```

### 1. 导出 DSL (Export)

将 Dify 上的应用导出为本地 YML。

```bash
# 基础导出（会提示输入密码，并以默认 tag '智能投标' 过滤应用）
./target/release/dify-console export \
  --url http://localhost:8080 \
  --email admin@example.com

# 携带密码，且导出所有 App 模式，自定义输出目录
./target/release/dify-console export \
  -u http://localhost:8080 \
  -e admin@example.com \
  -p my_secret_password \
  -o ./exported_dsl \
  -m all \
  -t "我的标签" \
  --include-secret

./target/release/dify-console export \
  -u http://localhost:8080 \
  -e admin@example.com \
  -p my_secret_password \
  -o ./exported_dsl \
  -m all \
  -t "我的标签" \
  --include-secret  
```

### 2. 导入/更新 DSL (Import)

支持单个文件导入，或者对目录进行批量导入（配合 mapping 文件）。

#### 单个文件导入
```bash
# 导入并创建一个新应用
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -f ./exported_dsl/workflows/app_id_123.yml
```

#### 批量目录导入
```bash
# 批量导入目录下的 YML 并记录/更新映射关系到 JSON 映射文件中
./target/release/dify-console import \
  -u http://localhost:8080 \
  -e admin@example.com \
  -d ./exported_dsl \
  -m ./exported_dsl/mapping.json
```

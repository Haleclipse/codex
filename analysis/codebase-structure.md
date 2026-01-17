# Codex 代码库结构分析

本文档详细阐述 Codex 项目的目录结构和各模块职责。

## 顶级目录结构

```
codex/
├── codex-rs/           # Rust 实现（主要维护版本）
├── codex-cli/          # TypeScript 实现（已弃用）
├── sdk/typescript/     # TypeScript SDK
├── shell-tool-mcp/     # Shell 工具 MCP 服务器
├── docs/               # 用户文档
├── scripts/            # 构建/发布脚本
├── third_party/        # 第三方依赖
├── .github/            # GitHub Actions 工作流
├── .devcontainer/      # 开发容器配置
├── .vscode/            # VS Code 配置
├── justfile            # 项目级 just 命令（默认工作目录为 codex-rs）
├── AGENTS.md           # Codex 代理指令文件
├── flake.nix           # Nix flake 配置
└── package.json        # pnpm 工作区根配置
```

---

## codex-rs/ - Rust 工作空间

这是 Codex 的主要实现，采用 Cargo workspace 组织。所有 crate 名称以 `codex-` 为前缀（如 `core/` 目录的 crate 名为 `codex-core`）。

### 核心 Crate

#### core/
**Crate**: `codex-core`

核心业务逻辑库，设计为供各种 Codex UI（TUI、CLI、IDE 扩展等）使用的可复用库。

**平台依赖**:
- macOS: 需要 `/usr/bin/sandbox-exec`
- Linux: 需要 `codex-linux-sandbox` 通过 arg0 机制可用
- 所有平台: 需要通过 arg1 支持 `--codex-run-as-apply-patch` 模拟虚拟 `apply_patch` CLI

#### cli/
**Crate**: `codex-cli`

CLI 多功能工具，作为统一入口提供各种子命令：
- `codex` - 启动 TUI
- `codex exec` - 非交互模式
- `codex mcp-server` - 启动 MCP 服务器
- `codex sandbox` - 测试沙箱
- `codex execpolicy` - 执行策略检查

#### tui/
**Crate**: `codex-tui`

基于 [Ratatui](https://ratatui.rs/) 的全屏终端用户界面。

**样式约定**（详见 `tui/styles.md`）:
- 使用 `Stylize` trait 辅助方法: `"text".dim()`, `.bold()`, `.cyan()`
- 简单 span: `"text".into()`
- 避免硬编码 `.white()` - 使用默认前景色
- 文本换行使用 `textwrap::wrap` 或 `tui/src/wrapping.rs`

#### tui2/
**Crate**: `codex-tui2`

实验性的第二代 TUI 实现。

#### exec/
**Crate**: `codex-exec`

无头（headless）CLI，用于自动化和非交互模式。通过 `codex exec PROMPT` 调用，Codex 会工作直到完成任务后退出。

#### protocol/
**Crate**: `codex-protocol`

协议类型定义，包括：
- 内部类型：`codex-core` 与 `codex-tui` 之间的通信
- 外部类型：与 `codex app-server` 配合使用

设计原则：最小依赖，避免包含业务逻辑。

#### common/
**Crate**: `codex-common`

跨 crate 共享的工具函数。不应放入 `core` 的通用功能放在这里。使用 feature flag 机制门控各功能模块。

---

### 服务器相关 Crate

#### app-server/
**Crate**: `codex-app-server`

为 VS Code 扩展等富客户端提供接口。采用 JSON-RPC 2.0 协议通过 stdio 进行双向通信。

**核心概念**:
- **Thread**: 用户与 Codex 代理的对话
- **Turn**: 对话的一个回合，通常从用户消息开始，以代理消息结束
- **Item**: 回合中的输入/输出单元（用户消息、代理推理、命令执行、文件变更等）

**主要 API**:
- `thread/start`, `thread/resume`, `thread/fork` - 线程管理
- `turn/start`, `turn/interrupt` - 回合控制
- `review/start` - 代码审查
- `command/exec` - 独立命令执行
- `skills/list` - 技能列表

#### app-server-protocol/
**Crate**: `codex-app-server-protocol`

App Server 的协议定义。

#### app-server-test-client/
测试 App Server 的客户端工具。

#### mcp-server/
**Crate**: `codex-mcp-server`

[Model Context Protocol](https://modelcontextprotocol.io/) 服务器实现。允许其他 MCP 客户端将 Codex 作为工具使用。

通过 `codex mcp-server` 启动，可使用 `@modelcontextprotocol/inspector` 测试。

#### exec-server/
**Crate**: `codex-exec-server`

包含两个可执行文件：

1. **codex-exec-mcp-server**: MCP 服务器，提供 `shell` 工具，在沙箱化的 Bash 中运行命令
2. **codex-execve-wrapper**: 拦截 `execve(2)` 调用，根据 `.rules` 决定：
   - `Run`: 在 Bash 内执行
   - `Escalate`: 在沙箱外特权运行
   - `Deny`: 拒绝执行

包含一个打补丁的 Bash（支持 `BASH_EXEC_WRAPPER` 环境变量）。

---

### MCP 相关 Crate

#### mcp-types/
**Crate**: `mcp-types`

Model Context Protocol 的类型定义，参考 [lsp-types](https://crates.io/crates/lsp-types) 设计。

#### rmcp-client/
**Crate**: `codex-rmcp-client`

RMCP 客户端实现。

---

### 沙箱与安全 Crate

#### linux-sandbox/
**Crate**: `codex-linux-sandbox`

Linux 平台的沙箱实现，使用 [Landlock](https://landlock.io/)。

产出：
- `codex-linux-sandbox` 独立可执行文件（与 Node.js CLI 捆绑）
- lib crate 暴露 `run_main()` 供 `codex-exec` 和 `codex` CLI 通过 arg0 调用

#### windows-sandbox-rs/
**Crate**: `codex-windows-sandbox`

Windows 平台的沙箱实现。

#### process-hardening/
**Crate**: `codex-process-hardening`

进程加固功能。

#### execpolicy/
**Crate**: `codex-execpolicy`

执行策略引擎，使用 Starlark 语法定义规则：

```starlark
prefix_rule(
    pattern = ["git", ["status", "diff"]],
    decision = "allow",  # allow | prompt | forbidden
    justification = "Git 读取操作是安全的",
    match = [["git", "status"]],
    not_match = [["git", "push"]],
)
```

CLI 命令：`codex execpolicy check --rules <file> <command>`

#### execpolicy-legacy/
**Crate**: `codex-execpolicy-legacy`

旧版执行策略匹配器。

---

### AI 提供商集成 Crate

#### chatgpt/
**Crate**: `codex-chatgpt`

ChatGPT 集成。

#### ollama/
**Crate**: `codex-ollama`

[Ollama](https://ollama.ai/) 本地模型集成。

#### lmstudio/
**Crate**: `codex-lmstudio`

[LM Studio](https://lmstudio.ai/) 集成。

#### backend-client/
**Crate**: `codex-backend-client`

后端 API 客户端。

#### responses-api-proxy/
**Crate**: `codex-responses-api-proxy`

Responses API 代理，包含 npm 包（`responses-api-proxy/npm/`）。

---

### 工具库 Crate (utils/)

#### utils/absolute-path/
**Crate**: `codex-utils-absolute-path`

绝对路径处理工具。

#### utils/cache/
**Crate**: `codex-utils-cache`

缓存工具。

#### utils/cargo-bin/
**Crate**: `codex-utils-cargo-bin`

Cargo 二进制文件定位工具。

**重要**: 在测试中优先使用 `codex_utils_cargo_bin::cargo_bin("...")` 而非 `assert_cmd::Command::cargo_bin(...)`，以支持 Bazel runfiles。

#### utils/git/
**Crate**: `codex-git`

Git 操作工具。

#### utils/image/
**Crate**: `codex-utils-image`

图像处理工具。

#### utils/json-to-toml/
**Crate**: `codex-utils-json-to-toml`

JSON 到 TOML 转换。

#### utils/pty/
**Crate**: `codex-utils-pty`

伪终端（PTY）工具。

#### utils/readiness/
**Crate**: `codex-utils-readiness`

就绪状态检查工具。

#### utils/string/
**Crate**: `codex-utils-string`

字符串处理工具。

---

### 其他 Crate

#### ansi-escape/
**Crate**: `codex-ansi-escape`

ANSI 转义序列处理。

#### apply-patch/
**Crate**: `codex-apply-patch`

补丁应用工具，用于文件编辑操作。测试 fixtures 在 `apply-patch/tests/fixtures/scenarios/`。

#### arg0/
**Crate**: `codex-arg0`

程序名称（argv[0]）处理。用于实现多工具二进制（如 `codex-linux-sandbox` 通过 arg0 调用）。

#### async-utils/
**Crate**: `codex-async-utils`

异步编程工具函数。

#### file-search/
**Crate**: `codex-file-search`

快速模糊文件搜索工具：
- 使用 [ignore](https://crates.io/crates/ignore)（ripgrep 使用的库）遍历目录
- 使用 [nucleo-matcher](https://crates.io/crates/nucleo-matcher) 进行模糊匹配

CLI: `codex file-search <PATTERN>`

#### feedback/
**Crate**: `codex-feedback`

用户反馈收集功能。

#### login/
**Crate**: `codex-login`

登录/认证功能。

#### keyring-store/
**Crate**: `codex-keyring-store`

系统密钥环存储。

#### otel/
**Crate**: `codex-otel`

[OpenTelemetry](https://opentelemetry.io/) 集成，用于可观测性。

#### stdio-to-uds/
**Crate**: `codex-stdio-to-uds`

stdio 到 Unix Domain Socket 的转换。

#### debug-client/
调试客户端工具。

#### codex-client/
**Crate**: `codex-client`

Codex 客户端库。

#### codex-api/
**Crate**: `codex-api`

Codex API 定义。

#### codex-backend-openapi-models/
**Crate**: `codex-backend-openapi-models`

后端 OpenAPI 模型定义。

#### cloud-tasks/
云任务相关功能。

#### cloud-tasks-client/
云任务客户端。

---

## codex-cli/ - TypeScript 实现（已弃用）

旧版 TypeScript 实现，现已被 codex-rs 取代。

```
codex-cli/
├── bin/            # 可执行入口
├── scripts/        # 构建/发布脚本
├── src/            # 源代码
└── package.json    # npm 包配置
```

**构建命令**:
```bash
corepack enable
pnpm install
pnpm build
node ./dist/cli.js
```

---

## sdk/typescript/

TypeScript SDK，封装 `codex` 二进制文件，通过 stdin/stdout 交换 JSONL 事件。

**安装**: `npm install @openai/codex-sdk`

**基本用法**:
```typescript
import { Codex } from "@openai/codex-sdk";

const codex = new Codex();
const thread = codex.startThread();
const turn = await thread.run("Diagnose the test failure");
console.log(turn.finalResponse);
```

**功能**:
- 流式响应: `runStreamed()`
- 结构化输出: `outputSchema` 参数
- 图像附件: `type: "local_image"`
- 线程恢复: `resumeThread(threadId)`

---

## shell-tool-mcp/

实验性 MCP 服务器，提供沙箱化的 `shell` 工具。

**特性**:
- 拦截 `execve(2)` 调用，精确知道被执行程序的完整路径
- 根据 `.rules` 文件决定命令处理方式：
  - `allow`: 在沙箱外执行（escalate）
  - `prompt`: 需人工批准（MCP elicitation）
  - `forbidden`: 拒绝执行

**配置** (`~/.codex/config.toml`):
```toml
[features]
shell_tool = false

[mcp_servers.shell-tool]
command = "npx"
args = ["-y", "@openai/codex-shell-tool-mcp"]
```

---

## docs/

用户文档目录。

主要文档：
- `getting-started.md` - 入门指南
- `config.md` - 配置说明
- `install.md` - 安装与构建
- `contributing.md` - 贡献指南
- `tui2/` - TUI2 相关文档

---

## scripts/

项目级脚本。

---

## third_party/

第三方依赖，如 wezterm 相关代码。

---

## .github/

GitHub 相关配置。

```
.github/
├── workflows/      # CI/CD 工作流
├── actions/        # 自定义 Actions
├── prompts/        # Prompt 模板
├── codex/          # Codex 相关配置
└── ISSUE_TEMPLATE/ # Issue 模板
```

---

## 配置文件说明

| 文件 | 说明 |
|------|------|
| `justfile` | just 命令定义，工作目录默认为 `codex-rs` |
| `AGENTS.md` | Codex 代理指令，定义代码规范和开发约定 |
| `flake.nix` | Nix flake 配置 |
| `package.json` | pnpm 工作区根配置 |
| `pnpm-workspace.yaml` | pnpm 工作区定义 |
| `.codespellrc` | 拼写检查配置 |
| `.prettierrc.toml` | Prettier 配置 |

---

## 沙箱机制总结

| 平台 | 机制 | 实现 |
|------|------|------|
| macOS | Apple Seatbelt | `/usr/bin/sandbox-exec` |
| Linux | Landlock + Seccomp | `codex-linux-sandbox` |
| Windows | WSL2 | 需要 WSL2 环境 |

**环境变量**:
- `CODEX_SANDBOX_NETWORK_DISABLED=1`: 沙箱中运行时设置
- `CODEX_SANDBOX=seatbelt`: macOS Seatbelt 子进程中设置

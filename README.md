# Kiro Gateway

> 基于 [kiro.rs](https://github.com/hank9999/kiro.rs) 二次开发，新增 Tauri 桌面端 GUI 管理界面。

一个用 Rust 编写的 Anthropic Claude API 兼容代理服务，将 Anthropic API 请求转换为 Kiro API 请求。

## 下载安装

从 [Releases](https://github.com/Zheng-up/Kiro-Gateway/releases) 页面下载对应系统的安装包：

| 系统        | 架构                     | 文件                                | 说明                  |
| ----------- | ------------------------ | ----------------------------------- | --------------------- |
| **Windows** | x64                      | `kiro-gateway_x.x.x_x64-setup.exe`  | NSIS 安装程序（推荐） |
| **Windows** | x64                      | `kiro-gateway_x.x.x_x64_en-US.msi`  | MSI 安装程序          |
| **macOS**   | Apple Silicon (M1/M2/M3) | `kiro-gateway_x.x.x_aarch64.dmg`    | ARM64 版本            |
| **macOS**   | Intel                    | `kiro-gateway_x.x.x_x64.dmg`        | x64 版本              |
| **Linux**   | x64                      | `kiro-gateway_x.x.x_amd64.deb`      | Debian/Ubuntu         |
| **Linux**   | x64                      | `kiro-gateway_x.x.x_amd64.AppImage` | 通用 Linux            |

### 安装注意事项

#### Windows

- 首次运行可能提示 "Windows 保护了你的电脑"，点击 "更多信息" → "仍要运行"
- 配置文件 `config.json` 和 `credentials.json` 放在 EXE 同级目录

#### macOS

- 首次运行提示 "无法验证开发者"：系统偏好设置 → 安全性与隐私 → 点击 "仍要打开"
- 或使用终端：`xattr -cr /Applications/kiro-gateway.app`

#### Linux

- AppImage 需要先添加执行权限：`chmod +x kiro-gateway_*.AppImage`
- 需要安装 `libwebkit2gtk-4.1`：`sudo apt install libwebkit2gtk-4.1-dev`

## 功能特性

- **Anthropic API 兼容**: 完整支持 Anthropic Claude API 格式
- **流式响应**: 支持 SSE (Server-Sent Events) 流式输出
- **Token 自动刷新**: 自动管理和刷新 OAuth Token
- **多凭据支持**: 支持配置多个凭据，按优先级自动故障转移
- **智能重试**: 单凭据最多重试 3 次，单请求最多重试 9 次
- **凭据回写**: 多凭据格式下自动回写刷新后的 Token
- **Thinking 模式**: 支持 Claude 的 extended thinking 功能
- **工具调用**: 完整支持 function calling / tool use
- **多模型支持**: 支持 Sonnet、Opus、Haiku 系列模型
- **桌面 GUI**: Tauri 桌面应用，可视化凭据管理

## 支持的 API 端点

| 端点                        | 方法 | 描述             |
| --------------------------- | ---- | ---------------- |
| `/v1/models`                | GET  | 获取可用模型列表 |
| `/v1/messages`              | POST | 创建消息（对话） |
| `/v1/messages/count_tokens` | POST | 估算 Token 数量  |
| `/api/admin/*`              | -    | 凭据管理 API     |

## 快速开始

### 方式一：使用桌面应用（推荐）

1. 从 [Releases](https://github.com/Zheng-up/Kiro-Gateway/releases) 下载对应系统的安装包
2. 安装并运行应用
3. 首次启动会自动创建 `config.json` 和 `credentials.json`
4. 在 GUI 中添加凭据并配置

### 方式二：从源码编译

```bash
# 克隆仓库
git clone https://github.com/Zheng-up/Kiro-Gateway.git
cd Kiro-Gateway

# 安装前端依赖
cd admin-ui && pnpm install && pnpm build && cd ..

# 编译 Tauri 应用
cd src-tauri && cargo tauri build
```

## 配置文件

### config.json

创建 `config.json` 配置文件（放在可执行文件同级目录）：

```json
{
  "host": "127.0.0.1",
  "port": 8990,
  "apiKey": "sk-kiro-rs-qazWSXedcRFV123456",
  "region": "us-east-1"
}
```

**完整配置选项：**

| 字段            | 类型   | 默认值      | 描述                                |
| --------------- | ------ | ----------- | ----------------------------------- |
| `host`          | string | `127.0.0.1` | 服务监听地址                        |
| `port`          | number | `8990`      | 服务监听端口                        |
| `apiKey`        | string | -           | 自定义 API Key（用于客户端认证）    |
| `region`        | string | `us-east-1` | AWS 区域                            |
| `kiroVersion`   | string | `0.8.0`     | Kiro 版本号（可选）                 |
| `machineId`     | string | 自动生成    | 自定义机器码（64 位十六进制，可选） |
| `proxyUrl`      | string | -           | HTTP/SOCKS5 代理地址（可选）        |
| `proxyUsername` | string | -           | 代理用户名（可选）                  |
| `proxyPassword` | string | -           | 代理密码（可选）                    |

### credentials.json

支持单对象格式（向后兼容）或数组格式（多凭据）。

#### 多凭据格式（推荐）

```json
[
  {
    "refreshToken": "第一个凭据的刷新token",
    "expiresAt": "2025-12-31T02:32:45.144Z",
    "authMethod": "social",
    "priority": 0
  },
  {
    "refreshToken": "第二个凭据的刷新token",
    "expiresAt": "2025-12-31T02:32:45.144Z",
    "authMethod": "idc",
    "clientId": "xxxxxxxxx",
    "clientSecret": "xxxxxxxxx",
    "priority": 1
  }
]
```

#### 单凭据格式

```json
{
  "refreshToken": "你的刷新token",
  "expiresAt": "2025-12-31T02:32:45.144Z",
  "authMethod": "social"
}
```

> **多凭据特性说明**：
>
> - 按 `priority` 字段排序，数字越小优先级越高（默认为 0）
> - 单凭据最多重试 3 次，单请求最多重试 9 次
> - 自动故障转移到下一个可用凭据
> - Token 刷新后自动回写到源文件

## 使用 API

```bash
curl http://127.0.0.1:8990/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk-kiro-rs-qazWSXedcRFV123456" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 1024,
    "messages": [
      {"role": "user", "content": "Hello, Claude!"}
    ]
  }'
```

### 流式响应

```json
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 1024,
  "stream": true,
  "messages": [...]
}
```

### Thinking 模式

```json
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 16000,
  "thinking": {
    "type": "enabled",
    "budget_tokens": 10000
  },
  "messages": [...]
}
```

## 模型映射

| Anthropic 模型 | Kiro 模型           |
| -------------- | ------------------- |
| `*sonnet*`     | `claude-sonnet-4.5` |
| `*opus*`       | `claude-opus-4.5`   |
| `*haiku*`      | `claude-haiku-4.5`  |

## 认证方式

支持两种 API Key 认证方式：

```
x-api-key: sk-your-api-key
```

或

```
Authorization: Bearer sk-your-api-key
```

## 项目结构

```
Kiro-Gateway/
├── config.json            # 配置文件（EXE 同级目录）
├── credentials.json       # 凭据文件（EXE 同级目录）
├── src-tauri/             # Rust 后端 + Tauri
│   ├── src/
│   │   ├── main.rs        # Tauri 入口
│   │   ├── kiro_server.rs # HTTP 服务
│   │   ├── admin/         # Admin API（凭据管理）
│   │   ├── anthropic/     # Anthropic API 兼容层
│   │   └── kiro/          # Kiro API 客户端
│   └── tauri.conf.json    # Tauri 配置
└── admin-ui/              # React 前端 (Vite + TypeScript)
    └── src/
        └── components/
            ├── dashboard.tsx
            ├── add-credential-dialog.tsx
            └── balance-dialog.tsx
```

## 技术栈

- **桌面框架**: [Tauri](https://tauri.app/) 2.x
- **Web 框架**: [Axum](https://github.com/tokio-rs/axum) 0.8
- **前端**: React + TypeScript + Vite + Tailwind CSS
- **异步运行时**: [Tokio](https://tokio.rs/)
- **HTTP 客户端**: [Reqwest](https://github.com/seanmonstar/reqwest)

## 注意事项

1. **凭证安全**: 请妥善保管 `credentials.json` 文件，不要提交到版本控制
2. **Token 刷新**: 服务会自动刷新过期的 Token，无需手动干预
3. **删除凭据**: 需要先禁用凭据才能删除
4. **配置文件**: 首次运行会自动在安装目录创建 `config.json` 和 `credentials.json`

## 开发与构建

### 环境要求

- Node.js 20+
- pnpm 9+
- Rust (stable)
- Tauri CLI 2.x

### 开发模式

```bash
# 1. 安装前端依赖
cd admin-ui
pnpm install

# 2. 启动开发服务器（带热更新）
cd ../src-tauri
cargo tauri dev
```

### 本地构建

```bash
# 1. 构建前端
cd admin-ui
pnpm build

# 2. 构建 Tauri 应用
cd ../src-tauri
cargo tauri build
```

构建产物位于 `src-tauri/target/release/bundle/` 目录。

## 命令行参数

```bash
# 使用默认配置（EXE 同级目录的 config.json 和 credentials.json）
./kiro-gateway

# 指定配置文件路径
./kiro-gateway -c /path/to/config.json --credentials /path/to/credentials.json

# 查看帮助
./kiro-gateway --help
```

## License

MIT

## 致谢

本项目基于以下优秀项目开发:

- [kiro.rs](https://github.com/hank9999/kiro.rs) - 原始项目
- [kiro2api](https://github.com/caidaoli/kiro2api)
- [proxycast](https://github.com/aiclientproxy/proxycast)

# Kiro Gateway

> 基于 [kiro.rs](https://github.com/hank9999/kiro.rs) 二次开发，新增 Tauri 桌面端 GUI。

一个用 Rust 编写的 Anthropic Claude API 兼容代理服务，将 Anthropic API 请求转换为 Kiro API 请求。提供桌面 GUI 管理界面，方便凭据管理和监控。

## 新增特性（相比原项目）

- **Tauri 桌面应用**: 打包为单文件 EXE，内置 Web UI
- **可视化凭据管理**: 添加/删除/导入凭据，查看余额
- **优先级调整**: 实时调整凭据优先级
- **去重检测**: 自动跳过重复凭据
- **配置文件自动创建**: 首次启动自动生成默认配置

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

## 支持的 API 端点

| 端点                        | 方法 | 描述             |
| --------------------------- | ---- | ---------------- |
| `/v1/models`                | GET  | 获取可用模型列表 |
| `/v1/messages`              | POST | 创建消息（对话） |
| `/v1/messages/count_tokens` | POST | 估算 Token 数量  |
| `/api/admin/*`              | -    | 凭据管理 API     |

## 快速开始

### 方式一：使用桌面应用（推荐）

1. 下载 Release 中的 EXE 文件
2. 将 `config.json` 和 `credentials.json` 放在 EXE 同级目录（首次启动会自动创建）
3. 双击运行

### 方式二：开发模式

```bash
# 进入 src-tauri 目录
cd src-tauri

# 运行开发模式
cargo tauri dev
```

### 方式三：编译打包

```bash
cd src-tauri
cargo tauri build
```

打包后的 EXE 位于 `src-tauri/target/release/` 目录。

## 配置文件

### config.json

```json
{
  "host": "127.0.0.1",
  "port": 8990,
  "apiKey": "sk-kiro-rs-qazWSXedcRFV123456",
  "region": "us-east-1"
}
```

| 字段       | 类型   | 默认值      | 描述                             |
| ---------- | ------ | ----------- | -------------------------------- |
| `host`     | string | `127.0.0.1` | 服务监听地址                     |
| `port`     | number | `8080`      | 服务监听端口                     |
| `apiKey`   | string | -           | 自定义 API Key（用于客户端认证） |
| `region`   | string | `us-east-1` | AWS 区域                         |
| `proxyUrl` | string | -           | HTTP/SOCKS5 代理地址（可选）     |

### credentials.json

支持单对象格式或数组格式（多凭据）：

```json
[
  {
    "refreshToken": "第一个凭据的刷新token",
    "authMethod": "social",
    "priority": 0
  },
  {
    "refreshToken": "第二个凭据的刷新token",
    "authMethod": "idc",
    "clientId": "xxxxxxxxx",
    "clientSecret": "xxxxxxxxx",
    "priority": 1
  }
]
```

> **多凭据特性说明**：
>
> - 按 `priority` 字段排序，数字越小优先级越高（默认为 0）
> - 自动故障转移到下一个可用凭据
> - 多凭据格式下 Token 刷新后自动回写到源文件

## 项目结构

```
kiro-gateway/
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
            ├── dashboard.tsx           # 主界面
            ├── add-credential-dialog.tsx
            └── balance-dialog.tsx
```

## 技术栈

- **桌面框架**: [Tauri](https://tauri.app/) 2.x
- **Web 框架**: [Axum](https://github.com/tokio-rs/axum) 0.8
- **前端**: React + TypeScript + Vite + Tailwind CSS
- **异步运行时**: [Tokio](https://tokio.rs/)

## 认证方式

支持两种 API Key 认证方式：

1. **x-api-key Header**

   ```
   x-api-key: sk-your-api-key
   ```

2. **Authorization Bearer**
   ```
   Authorization: Bearer sk-your-api-key
   ```

## 注意事项

1. **凭证安全**: 请妥善保管 `credentials.json` 文件，不要提交到版本控制
2. **Token 刷新**: 服务会自动刷新过期的 Token，无需手动干预
3. **删除凭据**: 需要先禁用凭据才能删除

## License

MIT

## 致谢

本项目基于以下优秀项目开发:

- [kiro.rs](https://github.com/hank9999/kiro.rs) - 原始项目
- [kiro2api](https://github.com/caidaoli/kiro2api)
- [proxycast](https://github.com/aiclientproxy/proxycast)

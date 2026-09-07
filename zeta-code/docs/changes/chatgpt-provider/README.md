# ChatGPT 初始入口

- 记录性质：一次工作及其证据；当前功能要求按下表进入长期规格。
- 状态：保留初始入口的测试证据；后续认证读取、首次创建及维护由共享后端记录。

## 本次记录

| 目标 | 本次行为变化 | 实施步骤 | 原验收 |
| --- | --- | --- | --- |
| [intent.md](intent.md) | [spec.md](spec.md) | [plan.md](plan.md) | [verification.md](verification.md) |

## 对应哪些长期文档

| 本次内容 | 维护位置 | 职责 |
| --- | --- | --- |
| 初始 AC-1 至 AC-5 | [ChatGPT 账户](../../spec/providers.md#chatgpt-账户) | 登录、取消、返回、退出及旧结果 |
| INV-1 与认证扩展 | [共享认证设计](../../../../docs/chatgpt-subscription.md) | 凭据管理模式与后端责任 |

## 后续认证工作

- 当前认证模式以[共享认证设计](../../../../docs/chatgpt-subscription.md)为准。
- 扩展要求与证据见[认证规格](../../../../zeta-rs/docs/changes/chatgpt-auth/spec.md)、[认证验收](../../../../zeta-rs/docs/changes/chatgpt-auth/verification.md)。
- 初始入口验收不代表真实设备登录或扩展后的认证路径已经全部验证。


返回[变更索引](../README.md)。

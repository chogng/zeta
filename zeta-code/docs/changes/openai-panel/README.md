# OpenAI 配置面板

- 记录性质：一次工作及其证据；当前功能要求按下表进入长期规格。
- 状态：当前规格是多页签、多连接及模型目录的八条要求；旧列表面板的五条要求及其验收另行原样保留；新版自动化证据已按当前候选记录，真实终端与账号仍待实测。

## 本次记录

| 目标 | 本次行为变化 | 实施步骤 | 当前验收入口 |
| --- | --- | --- | --- |
| [intent.md](intent.md) | [spec.md](spec.md) | [plan.md](plan.md) | [verification.md](verification.md) |

## 对应哪些长期文档

| 本次内容 | 维护位置 | 职责 |
| --- | --- | --- |
| AC-1、AC-2、AC-7 | [页签与字段](../../spec/providers.md#页签与字段)、[编辑与保存](../../spec/providers.md#编辑与保存) | 表单、导航与窄窗口 |
| AC-3、AC-4、AC-8、INV-1 | [编辑与保存](../../spec/providers.md#编辑与保存)、[凭据与持久化](../../spec/providers.md#凭据与持久化) | 草稿、连接身份、保存冲突及 Key 隔离 |
| AC-5、INV-2、INV-3 | [获取与选择模型](../../spec/providers.md#获取与选择模型) | 主动获取、协议、目录失效和错误 |
| AC-6 | [ChatGPT 账户](../../spec/providers.md#chatgpt-账户) | 账户流程与异步结果 |
| 后端配置与请求 | [供应商配置](../../../../docs/model-provider-config.md)、[TUI 架构](../../design/tui.md#后台请求与旧结果) | 共享配置、凭据和请求职责 |

## 版本与证据

| 阶段 | 保存在哪里 | 适用范围 |
| --- | --- | --- |
| 初始列表面板 | [原规格](spec-initial.md)取自 Git 提交 `dc9760352`；[原验收](verification-initial.md)的候选基线为 `702f2ea63` | 原五条 AC，不能解释成当前同编号要求 |
| 当前多连接改造 | [意图](intent.md)、[规格](spec.md)、[计划](plan.md) | 八条 AC 与三条 INV；对应[当前验收](verification.md)，需核对候选与要求版本 |

- 新版要求已在[长期供应商规格](../../spec/providers.md)整理，自动化验收已更新，真实终端与账号仍待实测。
- 旧规格曾在原目录被更新；本次从已提交版本提取原规格，并原样保存原验收，供直接查阅。未提交期间的每次改写无法靠当前文件重建。
- `spec-initial.md` 与 `verification-initial.md` 只读保存初始版本；现行行为以长期供应商规格为准。


返回[变更索引](../README.md)。

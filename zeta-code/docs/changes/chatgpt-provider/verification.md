# 初始入口验收记录

本页保留最初入口测试证据；后续只读复用、兼容文件创建和当前验证结果见[认证兼容验收](../../../../zeta-rs/docs/changes/chatgpt-auth/verification.md)。

日期：2026-09-07。范围为同目录 intent.md、spec.md 的当前版本。

候选基线：`d1771edc8a34fb83c7a5cba14b62f8e044e151db`。工作区同时存在其他 TUI、内存诊断改动，均保留。对全部已修改和未跟踪的 `.rs`、`.toml`、`.lock` 文件，按路径排序，以 `路径 + NUL + 内容（删除文件使用 <deleted>）+ NUL` 计算 SHA-256：`f429eec250f6b40bfa0a32e618bf0f66fd720ec5a973d35f7d7bdc80d40654ac`，共 77 个文件；本文不参与计算。

| 要求 | 实现 | 验证 |
| --- | --- | --- |
| AC-1 订阅入口 | 已实现 | Providers 列表测试及真实 PTY 导航通过；订阅与 API key 输入分开 |
| AC-2 账户与登录 | 已实现终端状态和协议调用 | 模拟账户、方案、设备码与精确 RPC 参数测试通过；真实 OpenAI 设备登录尚未验证 |
| AC-3 取消与完成 | 已实现 | 取消与完成竞态、响应之前收到完成通知、其他登录通知、失败重试，以及返回后重新进入均通过 |
| AC-4 退出 | 已实现 | RPC 测试确认只退出 openai-chatgpt，并重新读取账户；真实 OpenAI 登录账户的退出尚未验证 |
| AC-5 键盘与旧结果 | 已实现 | 应用键盘测试与 100×32、60×16 PTY 场景通过；页面关闭后不重开，较早 revision 不覆盖较新账户 |
| INV-1 凭据边界 | 已实现 | TUI 只调用账户 RPC，代码不读取 Codex、浏览器或系统凭据；PTY 服务端模型请求数为 0 |

实际命令：

- `just check zeta-tui`：前次通过。最终代码的正常产品构建由下方 CLI PTY 测试完成；交付前额外重跑 check 因另一项 workbench 构建持锁而取消，未把这次重跑记为通过。
- `just test zeta-tui --lib config::`：17 项通过。
- `just test zeta-tui --lib app::state::tests::chatgpt_subscription_keeps_pending_login_across_back_navigation_and_cancels_by_id -- --exact`：通过。
- `just test zeta-tui --lib client::notification::tests::account_notifications_reach_the_subscription_owner -- --exact`：通过。
- 修正多登录完成通知的先后顺序处理后，`just test zeta-tui --lib subscription`：25 项通过，其中包含订阅状态、应用键盘与通知测试，以及同名 Thread subscription 既有测试。
- `just test zeta-cli --test tui_real_scenarios actual_tui_opens_chatgpt_subscription_and_returns_to_providers -- --exact`：通过，真实 PTY，耗时 6.30 秒，不调用 OpenAI。
- `git diff --check`：通过。

初次验证曾被并行开发的内存诊断编译问题阻挡；修复 `Result::is_err_or` 与 CommandId 到 String 的转换，缺少 base64 依赖由工作区中的另一处修改补齐。PTY 场景起初沿用了已变化的 Welcome 文案和旧页签操作，按当前行为修正后通过。未修改其他场景或截图基线。审查为自查，没有独立审查。

本机登录态复用已纳入后续认证扩展；本页的代码指纹和测试仅对应初始入口版本，不代表扩展后的认证验证。

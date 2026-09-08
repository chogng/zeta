# 状态栏风格验收

2026-09-08；要求版本：[spec.md](spec.md)。基线 `7692f58601433c272ee779edbb015d4adb776067`，本次代码为该基线上的未提交改动。环境：macOS aarch64、Rust 1.98.0；本次为自查。

| 要求 | 实现与证据 | 结果 |
| --- | --- | --- |
| AC-1 | Config 键盘切换，四种语言标签，保留条目选择 | 包内测试通过 |
| AC-2 | 严格解析、往返保存、未知字段保留、过期修改拒绝、条目修改保留风格 | 包内测试与重启 PTY 通过 |
| AC-3 | Emoji、Plan / Context 进度条与主题颜色、当前 Thread 快照与模型容量、配置刷新保留有效数据 | 包内测试通过 |
| AC-4 | 10 / 4 格和文字压缩、零容量与零总数、组合 Emoji / 字符截断、Thread 切换 | 包内测试通过 |
| AC-5 | 包检查、相关回归与真实进程场景通过 | 通过 |

## 已执行的候选验证

- `just check zeta-tui`：通过（6m15s）；后续补齐配置刷新目录读取，最终产品构建由 PTY 场景覆盖。
- `just test zeta-tui --lib`：746 通过、3 失败、2 忽略。失败为两项预期字符快照变化和一项未分配终端导致的 `Device not configured`。
- 逐行核对并接受 `status_line_settings_with_accounting`（增加 Context 条目）与 `expressive_status_line`（图标、进度条与权限两行）。字符快照中的宽字占位与固定列尾部空格按渲染缓冲区保留。
- `just test zeta-tui --lib status`：76 通过，包括新增配置与数据刷新回归。
- 在 PTY 中运行 `just test zeta-tui --lib transcript_output_protocol_keeps_the_main_buffer_and_existing_history -- --exact terminal::session::tests::transcript_output_protocol_keeps_the_main_buffer_and_existing_history`：1 通过。
- 修改文档的本地链接检查：9 份文档，无失效路径；非快照文件 `git diff --check` 通过。

## 最终真实进程验证

`just test-tui actual_tui_status_line_style_persists_across_restart -- --nocapture`：通过。配套服务普通构建 5m30s，CLI 测试构建 1m45s，实际 PTY 场景 10.39s。验证 Config 切换、Emoji 输出、退出重启保留及切回简洁；断言没有模型调用。

本次 AC-1 至 AC-5 在上述 macOS 自动化范围内完成。24 份变更代码、构建文件与字符快照按路径排序，以 `路径 + NUL + 文件内容 + NUL` 拼接计算 SHA-256：`42b2c10b95ce366677cdcd621aaeca0a25750988643f3ddbdd2209da49e5240c`，文档不参与指纹。

其他终端、Windows、Linux 与屏幕阅读器尚未实测。

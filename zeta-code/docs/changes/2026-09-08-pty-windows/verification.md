# 验收

Windows 实测进行中。保留当前工作区其他改动；此前 Provider 自动化结果不冒充本次 PTY 结果。


## 调试过程

1. 移除 Unix 假 SSH 脚本后，Windows ConPTY 输出 `ESC[6n` 并等待应答。原宿主没有回送终端模拟器的应答，首次超时并在关闭时挂住；已停止该测试进程，接上应答和关闭顺序。
2. 一次重建遇到并行修改的 `TerminalSession::area` 可变借用编译错误；该处后来恢复可编译，未在本轮修改它。
3. 完整读取退出输出后发现测试 profile 路径导致 AF_UNIX 路径过长；缩短临时目录前缀，保留独立随机目录。
4. 新 CLI 使用了 9 月 5 日的旧 daemon，协议不同导致 endpoint identity 不同；新增 `just test-tui` 先构建匹配 daemon，并在测试环境明确指定其路径。
5. 失败诊断先复制终端输出再触发断言，避免持有 Mutex 时 panic 使读取线程一起失败；正常退出与 Drop 均关闭 PTY 并回收读取线程，Fixture 结束时停止自己的 daemon。

6. 新 daemon 的目录运行时需要 Windows 命令执行与沙盒设置程序。`just test-tui` 同时构建它们，fixture 显式绑定路径；没有关闭沙盒来绕过初始化。
7. Provider 全链路和账户页导航已通过。退出中反复 Ctrl+C 会打断终端恢复；现在仅在新的交互画面已稳定且终端仍处于交互模式时补发，不在恢复阶段发送。
8. 恢复场景读到了持久化会话，但 stdio 客户端 shutdown 等待 stdout EOF；Windows 存活进程持有管道时不能据此判断连接已关闭。客户端读取层新增可取消的管道读取，并补“写端仍活着时关闭读端”回归；待重新验证恢复场景。

9. 客户端 crate 禁止 unsafe；Windows 管道探测因此归入现有系统边界 `zeta-utils-pty::CancellablePipeReader`，app-server-client 仅包装 stdout。`just test zeta-utils-pty closing_stops_waiting_for_output_while_a_pipe_writer_is_still_alive -- --nocapture`：1 项通过，确认写端仍存活时读端也能关闭。


## Windows 最终场景结果

环境：Windows + ConPTY，真实 `zeta.exe`、同源 daemon、Windows 执行/沙盒辅助程序，独立 profile 和本地脚本化 HTTP 服务；没有真实账号或外部模型调用。

| 要求 | 命令过滤器（`just test zeta-cli --test tui_real_scenarios <过滤器> -- --nocapture`） | 结果 |
| --- | --- | --- |
| AC-1、AC-3 | `actual_tui_opens_chatgpt_subscription_and_returns_to_openai` | 1 项通过：启动、账户页导航、缩放、返回、退出 |
| AC-2、AC-3 | `actual_tui_provider_fields_save_cancel_and_fetch_models` | 1 项通过：中文/emoji、Enter 编辑/保存、Esc 取消、Key 掩码、只在手动 Fetch 后请求、成功/空目录/401/重试、缩放和退出 |
| AC-1、AC-3 | `actual_tui_pty_streams_utf8_resizes_exits_and_resumes` | 1 项通过：UTF-8 消息、实际 HTTP 请求、回答输出、缩放、退出、读取持久化会话、恢复且不再次调用模型 |
| AC-2 | `actual_tui_switches_language_and_persists_it` | 1 项通过：实际配置页面选择日语并持久化 |
| AC-1、AC-3 | `tui_process::` | 3 项通过：字符归一化、截断后输出 revision、分段光标查询应答 |

配套构建入口 `just test-tui actual_tui_pty_streams_utf8_resizes_exits_and_resumes -- --exact --nocapture` 通过；其他场景随后使用已构建的同一套程序定向运行。没有修改或新增字符快照，断言使用当前终端文字、持久化数据、HTTP 请求和进程退出。

未在本机执行 Unix 场景，也未运行其余整套历史 PTY 用例；明确依赖 `/bin/sh` 的沙盒测试保留 `cfg(unix)`。通过上述场景不表示所有终端宿主或真实服务账号已经验证。


## 最终检查与候选

- `just test zeta-app-server-client --lib session`：13 项通过。
- `just check zeta-cli --bin zeta`：通过，无警告。
- 管道关闭回归 1 项、宿主回归 3 项及真实 PTY 场景 4 项分别通过，总计 21 项定向测试；没有运行完整 Rust workspace。
- Git 基线 `67bc847a2e34491b11b7f65ce9293d345f457ab5`；本轮源码、依赖和命令入口见 [source.sha256](source.sha256)，清单 SHA-256 `6c0fd5d61a82ac9a65fb5a24a321e1dc91a5075038a2a89548e28dc7233a46af`。工作区还有其他任务的修改，未回退或格式化这些文件。
- 自查按键与帧证据、实际请求、配置、退出流程、FFI 归属及文档链接；未做独立审查。
- 本轮未引入新的模块文件或字符快照；旧假 SSH 启动逻辑已移除，真实 CLI/daemon 启动由同一个 TuiProcess 负责。

# 要求

- AC-1：Windows 与 Unix 共用 TuiProcess，直接启动真实 CLI，测试使用独立 profile、workspace、Codex 认证目录和本地 HTTP 服务。
- AC-2：通过实际字节输入验证 Enter 编辑、保存退出、Esc 取消和手动导航到 Fetch；检查界面、配置及 HTTP 请求。模型结果覆盖成功、空目录、失败及重试。
- AC-3：实际 PTY 缩放后仍可操作；UTF-8 输入保留；正常退出和异常清理不留下测试子进程或阻塞读线程。
- AC-4：运行并记录 Windows 定向场景与 CLI 构建检查；不将未运行的 Unix 场景报告为通过。平台专属沙盒测试显式限定平台。


实际测试发现 Windows stdio 输出管道可能在连接关闭后仍有存活写端。AC-3 补充：AppServerSession shutdown 必须停止自己的输出读取，不能无限等待其他写端退出；这属于客户端连接生命周期，由 app-server-client 修复并验证。

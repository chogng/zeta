# Process hardening

- 在产品入口读取密钥、创建线程之前禁止 core dump；Linux 禁止同用户调试附加，macOS 使用 `PT_DENY_ATTACH`。
- 进程构造阶段清理 `LD_*` 和 `DYLD_*`，避免子进程继承加载器注入变量；这不能撤销启动时已加载的动态库。
- Windows 限制 DLL 搜索目录并关闭崩溃弹窗；不承诺阻止管理员调试或系统级转储。
- 系统调用失败立即终止产品启动；FFI 和环境写入仅由本 crate 持有。
- 验证：`just test ash-process-hardening`，使用独立子进程检查环境与资源限制。

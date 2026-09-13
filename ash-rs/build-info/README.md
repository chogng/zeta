# Build info

- 统一提供版本、编译目标、Git commit 和构建 ID；运行时不调用 Git。
- 发布可以注入 `ASH_BUILD_COMMIT` 与 `ASH_BUILD_ID`；源码构建从当前 checkout 读取 commit。
- 默认构建 ID 是 commit 与 target 的 SHA-256，表示构建来源，不表示可执行文件字节或未提交改动。
- 无 Git 元数据的源码包明确返回空 commit 和构建 ID。
- 验证：`just test ash-build-info`。

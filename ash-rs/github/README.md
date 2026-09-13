# GitHub

- 通过有界、已认证的 GitHub CLI 请求读写 GitHub 对象，包括 Issue、评论、标签、负责人和 PR。
- 保留 GitHub 参数与返回值校验，不拥有 Agent、Thread、工作目录或模型调用。
- 不维护 Issue Workflow、assignment、领取租约、执行阶段或交付状态机。
- Issue 浏览缓存由 `ash-state` 维护；执行通过通用 Session/Agent API，Agent 使用获准的 Plugin 工具处理外部操作。

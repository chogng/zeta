---
description: Zeta targeted testing and command-line verification rules.
applyTo: "**/test/**,**/*_tests.rs,**/tests/**"
---

# Testing Guidelines

Use the smallest command that covers the changed behavior. Report a pass only after it completes successfully.

## 改动与配套内容同步

实现及其配套内容必须在同一次改动中更新，不能把测试通过当作同步工作已经完成。

| 改动 | 必须同步的内容与验证 |
| --- | --- |
| 新增或修改用户可见 TUI | 新增或更新对应 `insta` snapshot，并逐份审阅后接受；完整面板、标识列和确定性样式断言遵循 [TUI 规范](tui.instructions.md#learnings)。替换界面时保留对应行为和显示状态的覆盖，不能只删除旧基线。 |
| 修改 Agent 执行逻辑 | 列出受影响的主要逻辑与用户可见行为，新增或更新覆盖真实调用链的集成测试；复用现有测试设施，不以局部辅助函数测试代替流程验证。 |
| 修改配置类型或协议接口 | 同步调用方、校验、序列化测试和接口文档；更新该接口已有的 schema、生成类型及 fixtures。App Server 协议使用 `just generate-protocol`，并运行对应生成一致性测试。 |
| 修改依赖或编译期资源 | 同步所属项目的 lockfile 和受影响的构建、资源打包清单，并验证实际使用的构建入口。使用 Zeta 现有工具，不引入其他仓库专属的锁文件或命令。 |
| 替换、移动或删除实现 | 检查调用方、模块导出、快捷键、测试和文档，删除失去用途的代码；只被旧测试引用的接口也要重新判断是否仍有产品用途，不为保留退场实现而保留测试。 |

交付前检查本次测试和正常构建的 warning，处理由本次改动引入的 warning；不能通过 `allow(dead_code)`、伪造调用或新增仅为消除 warning 的测试来掩盖无用代码。构建退出码为零不代表验证完成；确实无法处理的 warning 必须说明原因和影响，不能宣称已全部完成。

## TypeScript frontend

- Name tests in behavior language and keep arrange, act, and assert easy to identify.
- Prefer comparing a complete result over many disconnected field assertions when the full value is the behavior.
- Do not export production helpers solely for tests.
- Prefer state, events, DOM semantics, accessibility, and geometry over screenshots.

## Rust

- Do not run `cargo check` or `cargo test` directly for routine validation. Use `just check <crate> [args]` or `just test <crate> [args]`; these commands select a package and configure the locked V8 files only when its dependency graph needs them.
- Start with the package that owns the changed behavior. A test-name filter does not limit workspace compilation, so always select a package and never run bare `cargo test <filter>` from the workspace root.
- Ask the user before running a complete workspace check or test suite. Escalate only after targeted validation passes and the change affects a shared workspace contract, or when the user explicitly requests full coverage.
- Do not add `--workspace`, `--all-targets`, or `--all-features` as routine validation expansion. Use package-scoped target or feature expansion only when the changed surface specifically requires it; combining expansion with a workspace-wide run requires the same explicit approval as a complete suite.

## app

- Assert state, commands, semantic identity, events, timing, output, and PTY lifecycle. Do not use screenshots or pixels as pass/fail evidence.
- Validate the running product with `just app`, `python3 -B build/cargo_with_v8.py run -p app`, or the built executable. Use `APP_SESSION_TRACE=1`; add `APP_SESSION_TRACE_FRAMES=1` only for frame timing.

## Learnings

* 修改代码前检查相关测试；行为或接口变化时同步更新测试，修复缺陷时补充能复现问题的回归测试，已有覆盖则复用。纯编译问题必须验证实际失败的非测试构建，涉及条件编译时检查受影响的构建配置，不能用单测通过代替正常构建通过。不要为凑改动机械新增测试、复制实现断言或削弱断言；交付时说明执行了哪些验证，未新增或修改测试时说明原因。
* Keep one incremental setting throughout a Rust validation round, and after a failure rerun only the failed test or target; switching artifact modes or rebuilding whole packages creates avoidable duplicate outputs and disk pressure.

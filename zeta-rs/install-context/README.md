# `zeta-install-context`

> 本 README 是 Zeta 进程安装布局与随包资源位置的实现契约。Tool execution 与参数约束由
> [`zeta-shell-command`](../shell-command/README.md) 维护；平台 sandbox 选择和 enforcement
> 由 [`docs/sandboxing.md`](../../docs/sandboxing.md) 维护。

`zeta-install-context` 在进程启动边界捕获 current executable、package layout、
`ZETA_RG_PATH`、`ZETA_BWRAP_PATH`、两个 Windows sandbox helper override 与 host `PATH`，
并为消费方提供稳定、有序的资源候选。它不验证或执行 binary，不拥有 Workspace、Tool policy、
approval、sandbox capability probe、下载、更新或安装 mutation。

## 布局与优先级

当前识别的 package layout：

```text
<package>/
├── zeta-package.json
├── bin/
│   └── zeta
├── zeta-path/
│   └── rg[.exe]
└── zeta-resources/
    ├── bwrap              # Linux
    ├── zeta-command-runner.exe              # Windows
    ├── zeta-windows-sandbox-setup.exe        # Windows
    └── skills/            # built-in Agent Skills
```

只有 executable 位于 `bin/`，且 package root 同时存在普通文件 `zeta-package.json`、
`zeta-path/` 与 `zeta-resources/` 时，才识别为 `InstallMethod::Package`；其他启动方式统一为
`Other`。metadata 的生成和 package 内容校验由
[`scripts/zeta_package`](../../scripts/zeta_package/README.md) 拥有，本 crate 只以其存在作为
layout marker，不解析或信任其中的字段。

`InstallContext::executable_candidates(ManagedExecutable::Ripgrep)` 返回互斥分支：

- 配置 `ZETA_RG_PATH` 时只返回 authoritative `ExplicitOverride`；
- 否则返回 `SearchPaths`，顺序为 `<package>/zeta-path/rg`、Zeta executable 同目录的 legacy
  candidate、启动时 host `PATH` candidates。

类型不会在 override 分支暴露 fallback paths，因此 override 无效时消费方必须直接失败。普通候选
可以逐个验证；第一个有效 candidate 必须 canonicalize 并冻结后才能进入 Tool binding。

`ManagedExecutable::Bubblewrap` 使用相同的 mutually-exclusive contract：配置
`ZETA_BWRAP_PATH` 时只返回 override；否则顺序为 package `zeta-resources/bwrap`、启动时 host
`PATH` candidates。它不会采用 executable sibling legacy path。

`ManagedExecutable::WindowsCommandRunner` 与 `WindowsSandboxSetup` 分别使用
`ZETA_WINDOWS_COMMAND_RUNNER_PATH` 和 `ZETA_WINDOWS_SANDBOX_SETUP_PATH`。无 override 时顺序
为 package `zeta-resources/`、启动时 host `PATH`；两个 helper 分开解析，但 Windows backend
只有在两者都通过精确 probe 后才可用。

## 公共契约

| Symbol | 职责 | 不承担 |
| --- | --- | --- |
| `InstallContext::current` | 捕获当前安装与环境 snapshot | 持续观察环境变化 |
| `PackageLayout` | 描述 metadata 与 package/bin/path/resources 路径 | 创建、解析或修改 package |
| `executable_candidates` | 生成有来源和优先级的候选 | executable 验证或 capability probe |
| `bundled_resource` | 返回现有普通 resource file | digest 验证或 materialization |
| `bundled_resource_directory` | 返回现有 resource directory | tree validation、Skill discovery |

调用关系：

```text
host composition
└─ InstallContext::current
   ├─ executable_candidates(Ripgrep)
   │  └─ RipgrepExecutable validation + canonical identity freeze
   ├─ executable_candidates(Bubblewrap)
   │  └─ LinuxSandbox validation + capability probe + canonical identity freeze
   ├─ executable_candidates(WindowsCommandRunner / WindowsSandboxSetup)
   │  └─ WindowsSandbox validation + protocol probe + canonical identity freeze
   └─ bundled_resource_directory("skills")
      └─ zeta-skills controlled BuiltIn source validation
```

如果该 crate 开始启动进程、解释 sandbox policy、下载资源或管理更新状态，说明 ownership 已经
漂移。具有平台生命周期的 helper 选择、复制和验证应留在对应 sandbox backend。

## 验证

```bash
cargo test --manifest-path zeta-rs/Cargo.toml -p zeta-install-context
cargo clippy --manifest-path zeta-rs/Cargo.toml \
  -p zeta-install-context --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/install-context:install-context-unit-tests
```

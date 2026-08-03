# `zeta-skills`

> 本 README 是 Skill S0 format、受控 source 与 metadata-only catalog 的当前实现契约。
> 跨 crate 的选择、激活、context、Plugin/MCP 组合与后续阶段由
> [`docs/skills.md`](../../docs/skills.md) 维护。

`zeta-skills` 当前实现 Phase S0：验证 built-in/user source root，流式读取并严格校验
Agent Skills `SKILL.md` frontmatter，计算完整文件 SHA-256 digest，并发布 deterministic、
immutable catalog snapshot。它不激活正文、不读取 references/assets/scripts、不执行命令，也不
拥有 config、watcher、App Server 或 Core integration。当前 App Server adapter、watcher 和
enablement overlay 属于 [`zeta-app-server`](../app-server/README.md)；该 adapter 只调用本 crate
的受控 source/catalog API，不改变这里的 ownership。

Repository-owned built-in content 位于 [`assets/`](assets)，打包后位于
`zeta-resources/skills/`。package host 通过
`InstallContext::bundled_resource_directory("skills")` 取得 candidate，再用
`SkillSourceRoot::built_in` 重新验证；打包 provenance 不替代本 crate 的内容校验。
当前正式内置内容只有 `skill-creator`；通用 code review/debugging 工作流没有因测试便利而成为
产品 built-in。

Catalog scanner 通过 [`zeta-file-identity`](../file-identity/README.md) 从已打开文件句柄取得
稳定 identity 与 hard-link count。`scan_skill_file` 在读取前绑定句柄 identity，读取后重新打开
受控 path 并比较 identity；因此 Windows 不依赖不稳定的标准库 `MetadataExt` API，也不会为了
跨平台编译而跳过 hard-link 或换文件检测。symlink、root containment、大小限制和 diagnostic
语义仍由本 crate 拥有。

## 公共契约

| Symbol | 当前职责 | 不承担 |
| --- | --- | --- |
| `zeta_protocol::SkillName` | Agent Skills 的 lowercase ASCII、数字、单连字符、1–64 字符 identity | display alias、Unicode normalization |
| `zeta_protocol::{SkillSourceId, SkillId}` | 跨 config/catalog/App Server 的 source-qualified stable identity | raw host path、版本选择 |
| `SkillSourceRoot` | host 注入并验证的 built-in/user real directory handle | config resolution、安装、immutability |
| `SkillCatalog::discover` | 对受控 roots 做首次 bounded metadata scan | arbitrary path search、recursive source search |
| `SkillCatalog::refresh` | 重扫并仅在 visible projection 改变时 bump generation | filesystem watching、safe-point scheduling |
| `SkillCatalogSnapshot::list/read` | deterministic metadata-only read API | `SKILL.md` body/content API |
| `SkillDiagnostic` | 隔离单 entry/source discovery failure | secret/body/private root 回传 |

`SkillSourceRoot` 的 canonical host path 是 private implementation state，其 `Debug` 输出也会隐藏
路径。调用方必须从已解析的 config/内建 release authority 构造 handle，不能把客户端提交的 path
直接转成 source。

## 文件、限制与真实调用路径

```text
SkillCatalog::discover / refresh
  → catalog::scanner::scan_sources
    → scan_source                     # direct children、1024-entry cap
      → scan_skill                    # containment/type/link checks
        → scan_skill_file             # stream hash；只 capture bounded frontmatter
        → format::parse_frontmatter   # YAML shape + Agent Skills field validation
  → SkillCatalogSnapshot::new
```

关键 limits：

| 项目 | 当前值 |
| --- | ---: |
| source direct entries | 1,024 |
| frontmatter | 16 KiB |
| frontmatter lines | 256 |
| one frontmatter line | 2 KiB |
| complete `SKILL.md` | 1 MiB |
| metadata entries | 64 |

`scan_skill_file` 对完整 `SKILL.md` bytes 流式 SHA-256，但最多只保留 16 KiB frontmatter；
Markdown body 不进入 catalog entry、diagnostic 或 snapshot debug projection。扫描前后检查
file identity、length、regular/single-link type；directory、manifest symlink 和 hard-linked
manifest 被拒绝。S0 不递归读取 optional directories，所以 scripts/references/assets 既不会被
加载，也不会产生副作用。

`format::validate_yaml_resource_shape` 在 `serde_yaml` 前限制 bytes/lines/line length/indent/flow
depth，并拒绝 YAML anchor、alias 与 tag control token。frontmatter 只接受规范字段，name 必须和
parent directory 一致；description、compatibility 和 metadata 另有 catalog memory bounds。
`allowed-tools` 只通过 `SkillMetadata::allowed_tools_hint` 暴露作者意图，绝不代表 approval。

## 失败、generation 与集成

一个坏 Skill 产生排序、去重后的 `SkillDiagnostic`，不会丢弃同 source 的有效 entry。source 在
handle 创建后消失会让下一次 refresh 发布 `SourceUnavailable` diagnostic。重复
`SkillSourceId` 是 host composition error，`SkillCatalog::discover` 返回 `DuplicateSource`。

entries 按 exact `SkillId` 排序，同名不同 source 同时保留。refresh 重新扫描事实；若 entries、
digest、compatibility、availability 与 diagnostics 完全不变，会复用同一个 `Arc` snapshot 和
generation。任何 consumer-visible 变化才发布下一 generation。

当前 App Server 已通过窄 adapter 组合 built-in/user roots、订阅 watcher，并以
`skills/list`/`skills/changed` 投影 catalog；per-Skill enablement 也由 config authority 与 adapter
叠加。共享 identity 位于 `zeta-protocol`，本 crate 只 re-export，不能重新定义一套。

本 crate 仍没有显式 activation、rooted resource resolver 或 safe-point scheduler。接入这些能力
应继续使用窄 adapter/module；如果 scanner 开始执行 script、读取 optional resource、解释 Plugin
grant，或 catalog 反向依赖 Core/App Server，表示 crate ownership 已经漂移。

## 验证

```bash
cargo test --manifest-path Cargo.toml -p zeta-skills
cargo clippy --manifest-path Cargo.toml \
  -p zeta-skills --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/skills:skills-unit-tests
```

tests 覆盖 identity/frontmatter contract、alias/depth/size bounds、metadata-only body exclusion、
source-qualified ordering/read、坏 entry 隔离、symlink/hardlink、digest refresh、no-op generation
与 private root diagnostic boundary。

# `zeta-file-icons`

> 本 README 是 Seti manifest、资源所有权和文件名解析行为的实现权威文档。
> Desktop 中的跨层归属见
> [`docs/zeta-desktop-architecture.md`](../../docs/zeta-desktop-architecture.md)。

`zeta-file-icons` 拥有 Rust 与 Desktop 共享的 Seti 文件图标资源，并为 Rust 客户端提供
manifest 校验和文件名解析能力。它不拥有 Explorer 状态、DOM 渲染、App Server RPC、终端
字体检测或 Nerd Font 二进制。

Seti 是随仓库代码一起构建的内部资源，不是需要跨版本协商的网络协议。因此这里不生成 JSON
Schema 或 TypeScript declarations；Desktop 直接从同步后的 JSON 推导其所需结构。

## 目录与所有权

```text
zeta-rs/file-icons/
├── src/
│   ├── lib.rs
│   ├── manifest.rs              # manifest shape、解析与引用校验
│   └── resolver.rs              # 文件名到 Seti icon ID 的确定性解析
└── seti/
    ├── manifest.json            # Seti associations 与 browser glyph metadata
    ├── seti.woff                # Desktop browser renderer 字体
    └── LICENSE.txt
```

`seti/manifest.json` 是唯一匹配数据源。修改 associations 后必须通过 manifest validation 与
resolver tests。Desktop 不得复制另一份 filename/extension association table。

## 公共接口

| Symbol | Contract |
| --- | --- |
| `SetiFileIconManifest` | manifest 根类型；拒绝未知字段 |
| `parse_seti_manifest` | JSON parse 后校验所有 association 引用 |
| `bundled_seti_manifest` | 惰性加载 checked-in manifest；资源损坏属于构建错误 |
| `resolve_file_icon` | exact filename → longest compound extension → language → default |
| `SetiColorScheme` | 显式选择 dark/light associations |

`ResolvedSetiFileIcon` 返回稳定的 Seti icon ID 和 browser artwork。Rust TUI 后续应以 icon
ID 接入 terminal Seti codepoint adapter，而不是加载 WOFF。

## 内部接口与调用路径

| Symbol | 可见性 | 责任 |
| --- | --- | --- |
| `validate_associations` | private | 遍历一组 association 并保留出错键路径 |
| `validate_reference` | private | 阻止 association 指向不存在的 icon definition |
| `resolve_specific` | private | 执行 filename、compound extension、language 优先级 |
| `extension_candidates` | private | 从最长复合扩展名开始产生候选项 |
| `language_id_for_extension` | private | 补充 Seti language ID 与常见扩展名的别名 |

```text
parse_seti_manifest
├── serde_json::from_str
└── SetiFileIconManifest::validate
    ├── validate_associations
    └── validate_reference

resolve_file_icon
└── resolve_specific
    ├── exact file name
    ├── extension_candidates
    └── language_id_for_extension
```

Renderer 可以拥有 glyph 绘制，但不能重新定义 Seti 匹配数据。当前 Rust 与 TypeScript
adapter 各自实现同一解析顺序，并由两侧单元测试锁定关键 case。

## Desktop 集成

`build/desktop/resources/syncFileIcons.ts` 在 Desktop build、dev、test 和 Renderer typecheck
之前把 manifest、WOFF 与 License 同步到 Desktop。也可以在仓库根目录单独执行：

```text
corepack pnpm sync:file-icons
```

直接验证 Rust crate：

```text
cargo test --manifest-path Cargo.toml -p zeta-file-icons
cargo fmt --manifest-path Cargo.toml -p zeta-file-icons -- --check
```

## 失败语义与当前限制

- JSON syntax 或未知字段错误返回 `SetiManifestError::InvalidJson`。
- association 指向缺失 definition 时返回带键路径的 `UnknownIconDefinition`。
- `bundled_seti_manifest` 只在 checked-in 资源未通过校验时 panic。
- manifest 包含 browser WOFF metadata，尚未包含 terminal Seti codepoint 或 ASCII
  fallback；纯 Rust 终端渲染是扩展点，不是当前能力。
- 真正接入 Rust TUI 后，应把两端解析 case 提升为共同消费的 conformance test data。

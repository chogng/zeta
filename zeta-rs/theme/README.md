# `zeta-theme`

> 本 README 是 `zeta-theme` 当前实现、接入边界和修改影响的 canonical 文档。图形界面主题系统见 [`docs/design-tokens.md`](../../docs/design-tokens.md)。Zeta Code TUI 主题由 [`zeta-code/tui`](../../zeta-code/tui/README.md) 独立拥有，不依赖本 crate。

`zeta-theme` 只服务 Rust 图形界面：

- 嵌入 Desktop registry 生成的颜色与尺寸 manifest，并将别名、覆盖和颜色变换解析为不可变 `ThemeSnapshot`；
- 严格解析 profile root 的 `themes/*.json`，隔离单文件错误，并按 `workbench.colorTheme` 选择主题；
- 为 `app` 提供枚举、预览、选择和原子保存接口，不包含终端 TUI 调色板、TUI 配置或 Ratatui 类型。

## 接口与边界

| Symbol | 职责 |
| --- | --- |
| `ThemeCatalog` | 读取版本化 manifest，解析默认值、别名、覆盖和颜色变换 |
| `ThemeDocument` | 严格解析最多 1 MiB、512 个覆盖项的图形界面用户主题 JSON |
| `ThemeSnapshot` | 保存完整 RGBA 与类型化标量尺寸表；不包含 DOM、WGPU 或 Ratatui 类型 |
| `ThemeLoader` | 有界读取 `configuration.json` 与 `themes/*.json`，选择主题并隔离错误文件 |
| `ThemeLoader::choices` / `preview` / `select` | 枚举有效主题、无副作用预览、验证后原子保存 `workbench.colorTheme` |
| `ThemeLoadOptions::with_default_entry` | 由 Rust 桌面产品选择 `zeta` 或 `app` 默认入口 |

Desktop 使用 TypeScript resolver；Rust 桌面端使用本 crate。两者读取同一 manifest、Schema 与 conformance fixture。`zeta-ui-theme` 将快照转换为组件调色板，组件本身不读取主题文件或 profile 配置。

## 执行路径

```text
ThemeLoader::embedded
└─ ThemeCatalog::embedded
   ├─ include_str!(resources/design-tokens/design-tokens.json)
   └─ include_str!(resources/design-tokens/theme-entries.json)

ThemeLoader::load(options)
├─ preference::read_preference(configuration.json / workbench.colorTheme)
├─ system → ThemeCatalog::built_in_entry(default_entry)
├─ read_theme_documents(themes/*.json)
│  └─ ThemeDocument::parse
├─ ThemeCatalog::resolve_document
└─ LoadedTheme { snapshot, diagnostics }

ThemeLoader::select(options, preference)
├─ ThemeLoader::preview
└─ preference::write_preference
   └─ zeta_utils_path::write_text_atomically
```

`Resolver` 是别名、默认值、覆盖、循环检测、变换深度、系数和透明度契约的唯一 owner。`read_preference` 只解释 `workbench.colorTheme`；`read_theme_documents` 只枚举非递归常规 JSON，按路径排序并限制为 128 个；`read_bounded_text` 在分配完整内容前把配置和主题文件限制为 1 MiB。

`theme-entries.json` 只包含 `zeta` 与 `app` 图形界面入口。配置为 `system` 时，所选入口跟随系统明暗方案；配置为 `<entry>-light`、`<entry>-dark` 或用户主题 ID 时固定到对应主题。

## 失败语义与接入义务

- embedded manifest 版本、重复 token 或解析失败会使 `ThemeLoader::embedded` 返回错误；宿主使用自己的最小安全调色板，不能猜 token。
- 未知默认入口产生诊断并回到 `zeta`；未知选择值回到调用方提供的默认入口。
- 一个用户主题文件失败只产生带路径的 `ThemeDiagnostic`，不会阻断其他主题。
- `ThemeSnapshot::required_color`、`required_size` 和 `required_pixel_size` 缺失或单位不匹配时返回 `ThemeError`；调用方必须原子保留上一份完整调色板或使用完整安全调色板。
- 新增图形界面 token 只在 Desktop registry 声明并重新生成 manifest；`tokens.rs` 只保存 Rust 调用方需要的稳定 ID 常量，不保存默认 RGB。
- 不得向本 crate 增加 `tui.*` token、TUI 主题入口、TUI 选择值或终端色彩降级。

## 修改与验证

修改 `document.rs` 必须同步用户主题 Schema 与 Desktop validator；修改 `catalog.rs` 的颜色数学或兼容映射必须更新 `theme-conformance.json` 并验证 TS/Rust 两端。修改入口默认值需更新 `resources/design-tokens/theme-entries.json`；修改 device path、文件上限或 `workbench.colorTheme` 行为必须同步 Desktop loader、本文与 `docs/design-tokens.md`。

当前高对比度保留独立明暗方案，但默认值继承对应 light/dark；Rust 桌面端在启动时读取外部主题文件，系统明暗事件只在选择 `system` 时重新生成内置快照。

```text
just test zeta-theme
bazel test //zeta-rs/theme:theme-unit-tests
```

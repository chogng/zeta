# `zeta-theme`

`zeta-theme` 是 Desktop、Native 与 TUI 共享主题 contract 的 Rust runtime。token catalog、JSON
Schema 和模板由 Desktop registry 编译到 [`resources/design-tokens`](../../resources/design-tokens)；
本 crate 嵌入同一 manifest，保留 alias/default graph，严格解析用户主题，并产生平台中立、不可变的
`ThemeSnapshot`。

| Symbol | 职责 |
| --- | --- |
| `ThemeCatalog` | 读取版本化共享 manifest，解析默认值、alias、覆盖和颜色变换 |
| `ThemeDocument` | 严格解析最多 1 MiB、512 个覆盖项的用户主题 JSON |
| `ThemeSnapshot` | 保存完整 resolved RGBA 与 typed scalar size token table；不包含 DOM、WGPU 或 Ratatui 类型 |
| `ThemeLoader` | 有界读取主题入口、device configuration 与 `themes/*.json`，隔离单文件错误并选择主题 |
| `ThemeLoader::choices` / `preview` / `select` | 枚举有效 built-in/user 主题；无副作用解析 preview；验证后原子保存 surface preference |
| `ThemeLoadOptions::with_default_entry` | 由产品启动组合选择 `zeta`、`zeta-code` 或 `app` 默认入口；组件和 token 不感知产品 |
| `ThemeSurface` | 选择 graphical 或 terminal device preference；不把 UI preference 放进 `zeta-config` |

Desktop 使用自己的 TypeScript resolver 和 CSS projection；Rust 与 TypeScript 通过同一 manifest、
Schema 和 parity fixture 保持一致。Native/TUI adapter 只能把 snapshot 投影为自己的 component
style，不能在本 crate 注册宿主状态或布局规则。TUI 可以只读取 token 子集并执行终端色彩能力降级。

## 执行路径与内部所有者

```text
ThemeLoader::embedded
└─ ThemeCatalog::embedded
   ├─ include_str!(resources/design-tokens/design-tokens.json)
   └─ include_str!(resources/design-tokens/theme-entries.json)

ThemeLoader::load(options)
├─ preference::read_preference(configuration.json)
├─ system preference → ThemeCatalog::built_in_entry(default_entry)
├─ read_theme_documents(themes/*.json)
│  └─ ThemeDocument::parse → validate schema/version/id/label/value bounds
├─ ThemeCatalog::resolve_document
│  ├─ normalize_legacy_editor_tokens
│  └─ Resolver::resolve_token / resolve_value
└─ LoadedTheme { snapshot, diagnostics }

ThemeLoader::select(options, preference)
├─ ThemeLoader::preview（先验证并解析 built-in/user theme，不写配置）
└─ preference::write_preference
   ├─ 保留其他 device-local values
   └─ zeta_utils_path::write_text_atomically
```

`Resolver` 是 alias/default/override graph、cycle path、transform depth、factor 与 transparency
contract 的唯一内部 owner；宿主不能再解析 token 引用。`read_preference` 只解释 device-local
`workbench.colorTheme` 与 `tui.colorTheme`；`preference::{read_preference,write_preference}` 是选择值
读取、保留未知 device values 与原子替换的内部 owner。`read_theme_documents` 只枚举非递归 regular JSON，按路径
排序并限制为 128 个。`read_bounded_text` 在分配完整文档前将配置和主题文件限制为 1 MiB。

`theme-entries.json` 只为同一 token catalog 提供数据化默认覆盖。`zeta`、`zeta-code` 与 `app`
不创建产品 token 或 resolver 分支；宿主只在 `ThemeLoadOptions` 选择入口。配置为 `system` 时，入口
跟随系统明暗方案；配置为 `<entry>-light`、`<entry>-dark` 或用户主题 ID 时固定到对应主题。

## 失败语义与接入义务

- embedded manifest 版本、重复 token 或解析失败会使 `ThemeLoader::embedded` 返回错误；这是构建
  contract 破坏，宿主应使用自己的最小安全 fallback，而不是猜 token。
- 未知宿主默认入口产生诊断并回退 `zeta`；未知 device preference 回退宿主所选的默认入口，而
  不是绕过入口回到全局颜色。
- 一个用户主题文件失败只产生带路径的 `ThemeDiagnostic`，不会阻断其他主题；选中的主题不可用时
  回退调用方提供的 system scheme。
- `ThemeSnapshot::required_color`、`required_size` 和 `required_pixel_size` 只用于宿主声明为必需的 token；缺失或单位不匹配返回 `ThemeError`，adapter
  必须原子地保留上一份完整 palette 或 fallback，不能部分应用。
- Native 选择 `ThemeSurface::Graphical`，TUI 选择 `ThemeSurface::Terminal`；TUI preference 缺失时
  loader 才回退 graphical preference。
- 新增 token 只在 Desktop registry 声明并重新生成 manifest；不得在 `tokens.rs` 加默认 RGB。
  `tokens.rs` 只是 Rust adapter 使用的稳定 ID 常量子集。

## 修改影响、测试与当前限制

修改 `document.rs` 必须同步用户主题 Schema 与 Desktop validator；修改 `catalog.rs` 的颜色数学或
legacy 映射必须同时更新共享 `theme-conformance.json` 并让 TS/Rust 两端通过。修改入口默认值需更新
`resources/design-tokens/theme-entries.json`；修改 device path、文件上限或 preference 优先级必须同步
Desktop loader、本文与 `docs/design-tokens.md`。

当前 snapshot 已包含共享 manifest 的颜色和类型化尺寸；Native/TUI 仍需要各自将尺寸投影为组件
style，不能让组件直接依赖 `zeta-theme`。Native/TUI 目前不监听外部主题文件，应用新 JSON 需要
重启对应进程；Native 会在系统明暗事件到达时重新选择 system snapshot。高对比度已保留独立
scheme，但默认值当前继承相应明暗方案。

```text
cargo test --manifest-path Cargo.toml -p zeta-theme
bazel test //zeta-rs/theme:theme-unit-tests
```

# Editor 对齐

仅在目标位于 `zeta-ts/src/zeta/editor` 时读取。Editor 与其他 TypeScript 范围使用相同的非对称对比规则，不启用额外的严格模式。

## 对应范围

- 本地根目录：`zeta-ts/src/zeta/editor`
- 上游根目录：`../vscode/src/vs/editor`
- 目标：VS Code 已有而 Zeta 缺失的生产文件和公开 API 持续补齐；Zeta 已有而 VS Code 没有的文件和公开 API 交由用户决定；对应实现承担相同公开契约、职责、状态所有权、生命周期和调用链。
- `editor` 可以依赖 `base`、`platform` 和自身更低运行环境的模块，不为“自包含”复制这些能力。`common` 只使用基础 JavaScript，DOM 进入 `browser`。

## 批量调查示例

用户指定 Editor 目录或同一目录下多个文件时，首次调查直接批量取得该目录双方文件集合、imports、exports、生产调用方和测试。输出过大时按结果维度拆分，不逐文件暂停。

修改前保存工作树来源基线，并对整个目标目录分类同路径、仅 Zeta、仅 VS Code 和大小写差异。仅 VS Code 项直接进入待补队列；尚无决定的仅 Zeta 项完成整批调查后一次性请求用户决定，已有决定的 Zeta 专属归属按确认职责继续使用。

## 专用检查

默认运行：

```powershell
node .agents/skills/vscode-api-alignment/scripts/check-editor-alignment.mjs
```

该入口依次执行：

- `verify-editor-api-ledger.mjs`：验证 118 项台账的唯一声明和已处理/待处理数量；
- `audit-editor-file-set.mjs`：审计整个 Editor 生产文件集合；
- `compare-editor-api-members.mjs`：按台账中的同名声明报告成员名差异；
- `git diff --check`；
- 仓库 `typecheck:stanza`；
- 检查运行前后是否新增未跟踪 `.js`。

需要完整逐文件输出时使用 `--full`。仅做修改前结构调查时可以使用 `--structure-only`。最终验证必须显式选择测试：无浏览器依赖的职责使用 `--test=unit`，浏览器职责使用 `--test=browser`，同时影响两类运行环境时使用 `--test=all`。不带 `--test` 的成功结果只代表结构与类型检查完成。

成员名报告只用于缩小人工核对范围。缺少的上游成员进入待补队列；尚无决定的仅 Zeta 成员必须请求用户决定，已有决定的 Zeta 专属 API 按确认职责检查。差异为零仍需检查签名、可见性、继承、owner、行为和调用链。

## TypeScript 输出边界

`zeta-ts/tsconfig.renderer.json` 必须保持 `noEmit: true`，Stanza 与扩展检查配置继承该不变量。只有负责生成产物的构建或测试配置可以启用输出，并必须把 `outDir` 明确设置在源码树外。

## 完成判定

一项 Editor API 只有在下列事实同时成立后才能从待处理表移入已处理表：

- 文件和 owning module 对应；
- 公开名称、签名和调用方对应；
- 状态、失效、调度、坐标、副作用、错误和释放职责对应；
- 生产调用真实经过对应 owner；
- 能直接触发该职责的本地独立行为测试通过，并在结果中列出测试名称；
- 专用检查入口通过。

仅 Zeta 文件或公开 API 必须先取得用户决定；确认作为 Zeta 专属归属后可以承接确认范围内的职责。调用方归零或 VS Code 不存在对应项都不能代替该决定。

当前 `test:editor:unit` 的编译范围还包含 Sessions 和聊天模块；若它被范围外类型错误阻塞，必须原样报告，不能把测试记为通过，也不能为了绕过错误使用跳过类型检查的执行方式。此时仍可运行独立的浏览器测试或更小的现有定向入口，但不能用无关测试替代受影响行为。

# Zeta 文档站

文档站把仓库中的权威 Markdown 转换为可搜索的网站；跨模块内容仍由 `../docs/*.md` 拥有，crate 实现契约仍由 `../zeta-rs/**/README.md` 拥有。

## 快速理解

| 修改内容 | 权威位置 | 需要运行的命令 |
| --- | --- | --- |
| 文档正文 | `../docs/` 或 crate README | `corepack pnpm --dir docs-site run check:docs` |
| 导航、标题和搜索数据 | `../build/docs/generateDocs.ts` | `corepack pnpm --dir docs-site run generate:docs` |
| 站点界面 | `app/`、`components/`、`lib/` | `corepack pnpm --dir docs-site run dev` |
| 构建与部署适配 | `../build/docs/`、`vite.config.ts`、`.openai/hosting.json` | `corepack pnpm --dir docs-site run build` |

## 本地开发

仓库使用一个 pnpm workspace 和根 `pnpm-lock.yaml`；不要在本目录运行 npm 或生成独立锁文件。

```bash
corepack pnpm install
corepack pnpm --dir docs-site run dev
```

Markdown 变化后重新运行 `corepack pnpm --dir docs-site run generate:docs`，或重启开发服务器。

## 验证

```bash
corepack pnpm --dir docs-site run check:docs
corepack pnpm --dir docs-site run typecheck
corepack pnpm --dir docs-site run test
```

规范检查独立于站点打包；测试会重新生成文档数据、构建生产 Worker，并验证文档索引的元数据和导航。

## 所有权

- `../build/docs/generateDocs.ts` 拥有来源发现、导航分组、标题、描述、目录和搜索文本生成。
- `../build/docs/checkDocs.ts` 拥有文档最低机械规范。
- `../build/docs/sitesVitePlugin.ts` 拥有 Sites 部署元数据和迁移文件的打包。
- `lib/markdown.ts` 拥有 Markdown 渲染和仓库链接改写。
- `components/docs-shell.tsx` 拥有导航、搜索、主题和响应式行为。
- `app/globals.css` 拥有文档站视觉系统。
- `../docs/documentation-guidelines.md` 拥有文档内容和结构规范。

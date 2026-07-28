# Zeta Desktop

Zeta Desktop 是 Zeta 的 Electron 客户端。

## 启动项目

首次运行时，先在仓库根目录 `zeta` 下安装依赖：

```bash
corepack pnpm install
```

安装完成后，在仓库根目录执行下面的命令启动桌面端：

```bash
corepack pnpm dev:desktop
```

这个命令会先编译 Rust CLI，然后启动 Vite、主进程、预加载脚本和 Electron。启动后不要关闭终端，停止服务可以按 `Ctrl+C`。

## 安装失败时

如果出现 `ERR_PNPM_ENOENT`、`electron_tmp` 或 Electron 目录 rename 错误，请先关闭正在运行的 Electron、Vite 和 Node 进程，然后在仓库根目录重建依赖：

```powershell
Remove-Item -LiteralPath .\node_modules -Recurse -Force
Remove-Item -LiteralPath .\desktop\node_modules -Recurse -Force
corepack pnpm install
corepack pnpm dev:desktop
```

这里只会删除 pnpm 生成的依赖目录，不会删除源码或 `pnpm-lock.yaml`。如果仍然失败，请暂时关闭占用 Electron 文件的杀毒软件实时扫描后重试。

## 常用命令

以下命令均可在仓库根目录执行：

```bash
# 构建桌面端
corepack pnpm build:desktop

# 只运行桌面端主进程测试
corepack pnpm --dir desktop test:main

# 检查 renderer 类型
corepack pnpm --dir desktop typecheck:renderer
```

如果 Electron 的依赖安装被 pnpm 拦截，请确认安装提示中的 `electron` 构建脚本已被允许。

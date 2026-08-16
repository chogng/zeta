import { readFile } from "node:fs/promises";
import { relative, resolve, sep } from "node:path";
import { normalizePath, type Plugin } from "vite";
import { analyzeHotReloadModule, type HotReloadModuleAnalysis, unsafeHotReloadChangeReason } from "./hotReloadAnalysis.ts";

const hotExportsName = "__zetaViteHotReloadExports";

export interface HotReloadPluginOptions {
  readonly desktopRoot?: string;
  readonly setupPath?: string;
}

/** Owns Vite's development-only bridge to the generic Renderer hot-reload runtime. */
export function hotReloadPlugin(options: HotReloadPluginOptions = {}): Plugin {
  const desktopRoot = resolve(options.desktopRoot ?? resolve(import.meta.dirname, "../.."));
  const setupPath = resolve(options.setupPath ?? resolve(import.meta.dirname, "setup-dev.ts"));
  const analyses = new Map<string, HotReloadModuleAnalysis>();
  return {
    name: "zeta-hot-reload",
    apply: "serve",
    transformIndexHtml: {
      order: "pre",
      handler: () => [{ tag: "script", attrs: { type: "module", src: `/@fs${normalizePath(setupPath)}` }, injectTo: "head-prepend" }],
    },
    transform: {
      order: "pre",
      handler(code, id) {
        const file = cleanModuleId(id);
        if (!file.endsWith(".ts")) return undefined;
        const analysis = analyzeHotReloadModule(code, file);
        if (!analysis.syntaxValid || analysis.classNames.length === 0) return undefined;
        if (code.includes(hotExportsName)) throw new Error(`Reserved hot-reload export is already declared: ${hotExportsName}`);
        analyses.set(file, analysis);
        return injectHotReloadBoundary(code, analysis.classNames, neutralModuleId(file, desktopRoot));
      },
    },
    async handleHotUpdate(context) {
      const file = cleanModuleId(context.file);
      const previous = analyses.get(file);
      if (!previous) return undefined;
      const code = context.read ? await context.read() : await readFile(file, "utf8");
      const next = analyzeHotReloadModule(code, file);
      if (!next.syntaxValid) return undefined;
      const reason = unsafeHotReloadChangeReason(previous, next);
      analyses.set(file, next);
      if (!reason) return undefined;
      const moduleId = neutralModuleId(file, desktopRoot);
      context.server.config.logger.info(`[hot-reload] Full reload: ${moduleId}: ${reason}`);
      context.server.ws.send({ type: "full-reload", path: "*" });
      return [];
    },
  };
}

function injectHotReloadBoundary(code: string, classNames: readonly string[], moduleId: string): string {
  const exports = classNames.join(", ");
  return `${code}\n\nconst ${hotExportsName} = { ${exports} };\nexport { ${hotExportsName} };\nif (import.meta.hot) {\n  const oldExports = import.meta.hot.data.$hotReloadExports ?? ${hotExportsName};\n  import.meta.hot.data.$hotReloadExports = oldExports;\n  const acceptNewExports = globalThis.$hotReload_applyNewExports?.({\n    oldExports,\n    newSrc: ${JSON.stringify(moduleId)},\n    config: { mode: "patch-prototype" },\n  });\n  if (acceptNewExports) {\n    import.meta.hot.accept(newModule => {\n      const newExports = newModule?.${hotExportsName};\n      if (!newExports || !acceptNewExports(newExports)) import.meta.hot?.invalidate("Hot-reload boundary became incompatible");\n    });\n  } else {\n    import.meta.hot.invalidate("No hot-reload runtime accepted this module");\n  }\n}\n`;
}

function cleanModuleId(id: string): string {
  return id.split("?", 1)[0];
}

function neutralModuleId(file: string, root: string): string {
  const path = relative(root, file);
  const normalized = path.split(sep).join("/");
  return normalized.startsWith("../") ? `external/${normalized.replace(/^\.\.\//u, "")}` : normalized;
}

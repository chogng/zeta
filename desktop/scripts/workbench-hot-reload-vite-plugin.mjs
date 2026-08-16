import { relative, resolve, sep } from "node:path";

const conventionalUiClassPattern = /\b(?:export\s+)?(?:abstract\s+)?class\s+([A-Za-z_$][\w$]*(?:Part|ViewPane|Widget))\s+extends\s+/gu;
const anyDerivedClassPattern = /\b(?:export\s+)?(?:abstract\s+)?class\s+([A-Za-z_$][\w$]*)\s+extends\s+/gu;
const explicitPrototypePatchMarker = "@zeta-hot-reload patch-prototype";

/**
 * Adds a VS Code-style prototype-patching HMR boundary to Renderer UI classes.
 * Incompatible updates invalidate the boundary so Vite performs a full reload.
 */
export function workbenchHotReloadPlugin(options = {}) {
  const root = resolve(options.root ?? resolve(import.meta.dirname, ".."));
  return {
    name: "zeta-workbench-hot-reload",
    apply: "serve",
    transform: {
      order: "pre",
      handler(code, id) {
        const file = cleanModuleId(id);
        if (!file.endsWith(".ts")) return undefined;
        const pattern = code.includes(explicitPrototypePatchMarker)
          ? anyDerivedClassPattern
          : conventionalUiClassPattern;
        const classNames = [...code.matchAll(pattern)].map(match => match[1]);
        if (classNames.length === 0) return undefined;

        const moduleId = neutralModuleId(file, root);
        const registrations = classNames.map(className => (
          `globalThis.$zetaHotReload_registerClass?.(${JSON.stringify(`${moduleId}#${className}`)}, ${className})`
        )).join(",\n    ");
        return `${code}\n\nif (import.meta.hot) {\n  const outcomes = [\n    ${registrations}\n  ];\n  if (outcomes.every(outcome => outcome && outcome !== "incompatible")) {\n    import.meta.hot.accept();\n  } else {\n    import.meta.hot.invalidate("Zeta UI class requires a full reload");\n  }\n}\n`;
      },
    },
  };
}

function cleanModuleId(id) {
  return id.split("?", 1)[0];
}

function neutralModuleId(file, root) {
  const path = relative(root, file);
  const normalized = path.split(sep).join("/");
  return normalized.startsWith("../") ? `external/${normalized.replace(/^\.\.\//u, "")}` : normalized;
}

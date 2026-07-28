import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const preloadPath = resolve(
  import.meta.dirname,
  "../dist/preload/src/zeta/base/parts/sandbox/electron-browser/preload.cjs",
);
const source = await readFile(preloadPath, "utf8");
const requiredModules = [...source.matchAll(
  /\brequire\(\s*(["'])(?<module>[^"']+)\1\s*\)/g,
)].map((match) => match.groups?.module);
const requireCallCount = [...source.matchAll(/\brequire\s*\(/g)].length;
const unsupportedModules = requiredModules.filter(
  (module) => module !== "electron",
);

if (requireCallCount !== requiredModules.length) {
  throw new Error("Sandbox preload contains a non-literal runtime import");
}

if (unsupportedModules.length > 0) {
  throw new Error(
    `Sandbox preload contains unsupported runtime imports: ${
      [...new Set(unsupportedModules)].join(", ")
    }`,
  );
}

if (!requiredModules.includes("electron")) {
  throw new Error("Sandbox preload does not import Electron");
}

if (/\bimport\s*\(/.test(source)) {
  throw new Error("Sandbox preload contains a dynamic runtime import");
}

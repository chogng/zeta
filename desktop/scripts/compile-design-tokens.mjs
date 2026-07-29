import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import path from "node:path";

const tsc = path.resolve("node_modules/typescript/bin/tsc");
const compilation = spawnSync(process.execPath, [tsc, "-p", "tsconfig.tokens.json"], { encoding: "utf8", stdio: "pipe" });
if (compilation.status !== 0) {
  process.stderr.write(compilation.stdout);
  process.stderr.write(compilation.stderr);
  process.exitCode = compilation.status ?? 1;
  throw new Error("Design token compiler failed to build");
}

const moduleUrl = pathToFileURL(path.resolve("dist/token-compiler/scripts/design-token-compiler.js"));
const { runDesignTokenCompiler } = await import(`${moduleUrl.href}?v=${Date.now()}`);
await runDesignTokenCompiler(process.argv.includes("--check"));

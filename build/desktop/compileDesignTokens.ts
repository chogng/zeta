import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const repositoryRoot = path.resolve(import.meta.dirname, "../..");
const desktopRoot = path.join(repositoryRoot, "desktop");
const tsc = path.join(desktopRoot, "node_modules/typescript/bin/tsc");
const compilation = spawnSync(process.execPath, [tsc, "-p", "tsconfig.tokens.json"], { cwd: desktopRoot, encoding: "utf8", stdio: "pipe" });
if (compilation.status !== 0) {
  process.stderr.write(compilation.stdout);
  process.stderr.write(compilation.stderr);
  process.exitCode = compilation.status ?? 1;
  throw new Error("Design token compiler failed to build");
}

const moduleUrl = pathToFileURL(path.join(repositoryRoot, ".build/desktop/token-compiler/build/desktop/resources/designTokenCompiler.js"));
const { runDesignTokenCompiler } = await import(`${moduleUrl.href}?v=${Date.now()}`);

export async function compileDesignTokens(check = false) {
  await runDesignTokenCompiler(check);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await compileDesignTokens(process.argv.includes("--check"));
}

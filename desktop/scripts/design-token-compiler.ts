import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import "../src/zeta/platform/theme/common/colorTheme.js";
import { compileDesignTokenArtifacts } from "../src/zeta/platform/theme/common/tokenCompiler.js";

const outputs = {
  manifest: "design-tokens.json",
  schema: "design-tokens.schema.json",
  catalog: "design-tokens.md",
  userThemeSchema: "color-theme.schema.json",
  userThemeTemplate: "color-theme.template.json",
} as const;

export async function runDesignTokenCompiler(check: boolean): Promise<void> {
  const outputDirectory = resolve("../resources/design-tokens");
  const artifacts = compileDesignTokenArtifacts();
  if (!check) await mkdir(outputDirectory, { recursive: true });
  const stale: string[] = [];
  for (const [artifact, filename] of Object.entries(outputs)) {
    const expected = artifacts[artifact as keyof typeof artifacts];
    const output = resolve(outputDirectory, filename);
    if (check) {
      const actual = await readFile(output, "utf8").catch(() => "");
      if (actual !== expected) stale.push(filename);
    } else {
      await writeFile(output, expected, "utf8");
    }
  }
  if (stale.length > 0) throw new Error(`Generated design token artifacts are stale: ${stale.join(", ")}. Run 'pnpm tokens:generate'.`);
}

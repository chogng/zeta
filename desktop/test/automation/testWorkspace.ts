import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

export interface TestWorkspace {
  readonly directory: string;
  readonly file: string;
}

/** Creates an isolated folder and text file for App Server-backed UI tests. */
export async function createTestWorkspace(): Promise<TestWorkspace> {
  const directory = await mkdtemp(join(tmpdir(), "zeta-playwright-workspace-"));
  const file = join(directory, "main.ts");
  await writeFile(file, "const value = 1;\n", "utf8");
  return { directory, file };
}

/** Removes one test workspace created by {@link createTestWorkspace}. */
export async function disposeTestWorkspace(workspace: TestWorkspace): Promise<void> {
  await rm(workspace.directory, { force: true, recursive: true });
}

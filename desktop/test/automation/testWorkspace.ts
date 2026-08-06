import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createAcademicDocumentSchema } from "../../src/zeta/editor/gama/contrib/academic/common/schema.js";
import { serializeDocument } from "../../src/zeta/editor/gama/common/model/documentSerialization.js";

export interface TestWorkspace {
  readonly directory: string;
  readonly file: string;
  readonly academicFile: string;
}

/** Creates an isolated folder and text file for App Server-backed UI tests. */
export async function createTestWorkspace(): Promise<TestWorkspace> {
  const directory = await mkdtemp(join(tmpdir(), "zeta-playwright-workspace-"));
  const file = join(directory, "main.ts");
  const academicFile = join(directory, "paper.zeta-academic");
  await writeFile(file, "const value = 1;\n", "utf8");
  await writeFile(academicFile, createAcademicDocument(), "utf8");
  return { directory, file, academicFile };
}

/** Removes one test workspace created by {@link createTestWorkspace}. */
export async function disposeTestWorkspace(workspace: TestWorkspace): Promise<void> {
  await rm(workspace.directory, { force: true, recursive: true });
}

function createAcademicDocument(): string {
  const schema = createAcademicDocumentSchema();
  const title = schema.createNode("title", {
    content: [schema.createNode("heading", { content: [schema.createText("Academic draft")] })],
  });
  const abstract = schema.createNode("abstract", {
    content: [schema.createNode("paragraph", { content: [schema.createText("A structured document for editor tests.")] })],
  });
  const source = schema.createNode("textBlock", {
    attrs: { language: "typescript" },
    content: [schema.createText("const paper = 1;")],
  });
  return serializeDocument(schema.createDocument([title, abstract, source]), schema);
}

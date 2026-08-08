import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createAcademicDocumentSchema } from "../../src/zeta/editor/gama/contrib/academic/common/schema.js";
import { serializeDocument } from "../../src/zeta/editor/gama/common/model/documentSerialization.js";

export interface TestWorkspace {
  readonly directory: string;
  readonly file: string;
  readonly rustFile: string;
  readonly academicFile: string;
  readonly pdfFile: string;
}

/** Creates an isolated folder and text file for App Server-backed UI tests. */
export async function createTestWorkspace(): Promise<TestWorkspace> {
  const directory = await mkdtemp(join(tmpdir(), "zeta-playwright-workspace-"));
  const file = join(directory, "main.ts");
  const rustFile = join(directory, "main.rs");
  const academicFile = join(directory, "paper.zeta-academic");
  const pdfFile = join(directory, "paper.pdf");
  await writeFile(file, "const value = 1;\n", "utf8");
  await writeFile(rustFile, "fn main() {\n    work();\n}\n", "utf8");
  await writeFile(academicFile, createAcademicDocument(), "utf8");
  await writeFile(pdfFile, createPdfDocument());
  return { directory, file, rustFile, academicFile, pdfFile };
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

function createPdfDocument(): Uint8Array {
  const header = "%PDF-1.4\n";
  const stream = "BT /F1 24 Tf 72 72 Td (Zeta PDF) Tj ET\n";
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    `<< /Length ${Buffer.byteLength(stream, "utf8")} >>\nstream\n${stream}endstream`,
  ];
  let content = header;
  const offsets = [0];
  for (const [index, object] of objects.entries()) {
    offsets.push(Buffer.byteLength(content, "utf8"));
    content += `${index + 1} 0 obj\n${object}\nendobj\n`;
  }
  const xrefOffset = Buffer.byteLength(content, "utf8");
  content += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets.slice(1)) content += `${offset.toString().padStart(10, "0")} 00000 n \n`;
  content += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  return Buffer.from(content, "utf8");
}

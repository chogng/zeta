import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../src/zeta/base/common/uri.js";
import { FileKind, type IFileService } from "../src/zeta/platform/files/common/files.js";
import { TextFileContentSource, TextFileService } from "../src/zeta/workbench/services/textfile/common/textFileService.js";

test("TextFileService uses bootstrap content without reading the workspace", async () => {
  const files = new TestFileService("workspace");
  const service = new TextFileService(files);
  const resource = URI.file("C:\\project\\main.ts");

  const content = await service.resolve({ resource, bootstrapText: "bootstrap" }, new AbortController().signal);

  assert.equal(content.text, "bootstrap");
  assert.equal(content.source, TextFileContentSource.Bootstrap);
  assert.equal(files.readCount, 0);
});

test("TextFileService reads missing bootstrap content and observes cancellation", async () => {
  const files = new TestFileService("workspace");
  const service = new TextFileService(files);
  const resource = URI.file("C:\\project\\main.ts");
  const content = await service.resolve({ resource }, new AbortController().signal);
  assert.equal(content.text, "workspace");
  assert.equal(content.source, TextFileContentSource.FileSystem);

  const cancelled = new AbortController();
  cancelled.abort("closed");
  await assert.rejects(service.resolve({ resource }, cancelled.signal), error => (error as Error).name === "CancellationError");
  assert.equal(files.readCount, 1);
});

test("TextFileService cancels an in-flight file read without publishing late content", async () => {
  const pending = deferred<string>();
  const files = new TestFileService(pending.promise);
  const service = new TextFileService(files);
  const controller = new AbortController();
  const resolving = service.resolve({ resource: URI.file("C:\\project\\slow.ts") }, controller.signal);

  controller.abort("closed");
  await assert.rejects(resolving, error => (error as Error).name === "CancellationError");
  pending.resolve("late");
  assert.equal(files.readCount, 1);
});

test("TextFileService preserves file-system failures", async () => {
  const failure = new Error("unreadable");
  const service = new TextFileService(new TestFileService(Promise.reject(failure)));

  await assert.rejects(
    service.resolve({ resource: URI.file("C:\\project\\main.ts") }, new AbortController().signal),
    error => error === failure,
  );
});

class TestFileService implements IFileService {
  readCount = 0;

  constructor(private readonly content: string | Promise<string>) {}

  async stat(resource: URI) {
    return {
      resource,
      kind: FileKind.File,
      sizeBytes: typeof this.content === "string" ? this.content.length : 0,
      readonly: false,
      modifiedAtMillis: undefined,
    };
  }

  async readDirectory() {
    return [];
  }

  async readFile() {
    this.readCount += 1;
    return await this.content;
  }
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(resolver => {
    resolve = resolver;
  });
  return { promise, resolve };
}

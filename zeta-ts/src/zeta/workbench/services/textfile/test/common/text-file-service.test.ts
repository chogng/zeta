import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../../../base/common/uri.js";
import { FileKind, FileRevisionConflictError, type IFileService, type IFileWriteRequest } from "../../../../../platform/files/common/files.js";
import { TextFileContentSource, TextFileSaveConflictError, TextFileService } from "../../../../../workbench/services/textfile/common/textFileService.js";

test("TextFileService uses bootstrap content without reading the workspace", async () => {
  const files = new TestFileService("workspace");
  const service = new TextFileService(files);
  const resource = URI.file("C:\\project\\main.ts");

  const content = await service.resolve({ resource, bootstrapText: "bootstrap" }, new AbortController().signal);

  assert.equal(content.text, "bootstrap");
  assert.equal(content.source, TextFileContentSource.Bootstrap);
  assert.equal(content.revision, undefined);
  assert.equal(files.readCount, 0);
});

test("TextFileService reads missing bootstrap content and observes cancellation", async () => {
  const files = new TestFileService("workspace");
  const service = new TextFileService(files);
  const resource = URI.file("C:\\project\\main.ts");
  const content = await service.resolve({ resource }, new AbortController().signal);
  assert.equal(content.text, "workspace");
  assert.equal(content.source, TextFileContentSource.FileSystem);
  assert.equal(content.revision, "revision-1");

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

test("TextFileService writes text and observes cancellation", async () => {
  const files = new TestFileService("workspace");
  const service = new TextFileService(files);
  const resource = URI.file("C:\\project\\main.ts");

  const saved = await service.save({ resource, text: "saved", expectedRevision: "revision-1" }, new AbortController().signal);
  assert.deepEqual(files.writes, [{ resource, content: "saved", expectedRevision: "revision-1" }]);
  assert.equal(saved.revision, "revision-2");

  const cancelled = new AbortController();
  cancelled.abort("closed");
  await assert.rejects(service.save({ resource, text: "ignored" }, cancelled.signal), error => (error as Error).name === "CancellationError");
  assert.equal(files.writes.length, 1);
});

test("TextFileService maps conditional file-write conflicts to its editor-facing error", async () => {
  const files = new TestFileService("workspace");
  files.rejectWritesWithRevisionConflict = true;
  const service = new TextFileService(files);
  const resource = URI.file("C:\\project\\main.ts");

  await assert.rejects(service.save({ resource, text: "saved", expectedRevision: "stale" }, new AbortController().signal), TextFileSaveConflictError);
});

class TestFileService implements IFileService {
  readCount = 0;
  readonly writes: IFileWriteRequest[] = [];
  rejectWritesWithRevisionConflict = false;
  readonly onDidChangeFiles = () => ({
    dispose() {},
    [Symbol.dispose]() {},
  });

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

  async readFile(resource: URI) {
    this.readCount += 1;
    return { resource, content: await this.content, revision: "revision-1" };
  }

  async readFileBytes(resource: URI) {
    return { resource, bytes: new Uint8Array(), revision: "revision-1" };
  }

  async writeFile(request: IFileWriteRequest) {
    if (this.rejectWritesWithRevisionConflict) throw new FileRevisionConflictError(request.resource);
    this.writes.push(request);
    return {
      stat: {
        resource: request.resource,
        kind: FileKind.File,
        sizeBytes: request.content.length,
        readonly: false,
        modifiedAtMillis: undefined,
      },
      revision: "revision-2",
    };
  }

  async createFile(): Promise<never> { throw new Error("Text file tests do not create empty files"); }
  async rename(): Promise<never> { throw new Error("Text file tests do not rename files"); }
  async delete(): Promise<never> { throw new Error("Text file tests do not delete files"); }
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(resolver => {
    resolve = resolver;
  });
  return { promise, resolve };
}

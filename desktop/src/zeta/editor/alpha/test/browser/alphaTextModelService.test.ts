import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../base/common/event.js";
import { URI } from "../../../../base/common/uri.js";
import { type IFileChangeEvent } from "../../../../platform/files/common/files.js";
import { TextFileContentSource, type ITextFileService, type TextFileSaveRequest } from "../../../../workbench/services/textfile/common/textFileService.js";
import { AlphaTextModelConflictError, AlphaTextModelService } from "../../browser/alphaTextModelService.js";
import { TextPosition, TextRange } from "../../common/text.js";

test("Alpha text model service shares one model and preserves edits across panes", async () => {
  using models = new AlphaTextModelService();
  const textFiles = new TestTextFileService("from disk");
  const input = { resource: URI.file("C:\\project\\main.ts"), initialText: "bootstrap" };
  const first = await models.acquire(input, textFiles, new AbortController().signal);
  const second = await models.acquire({ ...input, initialText: "stale" }, textFiles, new AbortController().signal);

  assert.equal(first.model, second.model);
  assert.equal(first.model.getText(), "bootstrap");
  assert.equal(textFiles.resolveCount, 1);
  first.model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 9)),
    text: "edited",
  }]);
  assert.equal(second.model.getText(), "edited");

  first.dispose();
  assert.equal(second.model.getText(), "edited");
  second.dispose();
  assert.throws(() => second.model.getText(), /disposed/);
});

test("Alpha text model acquisition delegates absent bootstrap content and observes cancellation", async () => {
  using models = new AlphaTextModelService();
  const textFiles = new TestTextFileService("from disk");
  const resource = URI.file("C:\\project\\main.ts");
  const reference = await models.acquire({ resource }, textFiles, new AbortController().signal);
  assert.equal(reference.model.getText(), "from disk");
  reference.dispose();

  const cancelled = new AbortController();
  cancelled.abort();
  await assert.rejects(models.acquire({ resource }, textFiles, cancelled.signal), error => (error as Error).name === "CancellationError");
});

test("Alpha text model references track dirty content, save snapshots, and explicitly revert", async () => {
  using models = new AlphaTextModelService();
  const textFiles = new TestTextFileService("from disk");
  const reference = await models.acquire({ resource: URI.file("C:\\project\\main.ts") }, textFiles, new AbortController().signal);
  let dirtyChanges = 0;
  using listener = reference.onDidChangeDirty(() => dirtyChanges += 1);

  reference.model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 4)),
    text: "saved",
  }]);
  assert.equal(reference.isDirty, true);
  assert.equal(dirtyChanges, 1);

  await reference.save(textFiles, new AbortController().signal);
  assert.deepEqual(textFiles.savedTexts, ["saved disk"]);
  assert.equal(reference.isDirty, false);
  assert.equal(dirtyChanges, 2);

  reference.model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 5)),
    text: " locally",
  }]);
  assert.equal(reference.isDirty, true);
  textFiles.setText("external\r\ncontent");
  await reference.revert(textFiles, new AbortController().signal);
  assert.equal(reference.model.getText(), "external\ncontent");
  assert.equal(reference.model.canUndo, false);
  assert.equal(reference.model.canRedo, false);
  assert.equal(reference.isDirty, false);
  assert.equal(dirtyChanges, 4);
});

test("Alpha text model save tolerates its final reference closing before I/O completes", async () => {
  using models = new AlphaTextModelService();
  const pending = deferred<void>();
  const textFiles: ITextFileService = {
    onDidChangeFiles: inertFileChanges,
    async resolve(request) {
      return {
        resource: request.resource,
        text: "from disk",
        source: TextFileContentSource.FileSystem,
      };
    },
    async save() {
      await pending.promise;
    },
  };
  const reference = await models.acquire({ resource: URI.file("C:\\project\\main.ts") }, textFiles, new AbortController().signal);
  reference.model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "edited ",
  }]);
  const saving = reference.save(textFiles, new AbortController().signal);
  reference.dispose();
  pending.resolve();
  await saving;
});

test("Alpha text model preserves the source CRLF convention when saving", async () => {
  using models = new AlphaTextModelService();
  const textFiles = new TestTextFileService("first\r\nsecond");
  const reference = await models.acquire({ resource: URI.file("C:\\project\\main.ts") }, textFiles, new AbortController().signal);
  assert.equal(reference.model.getText(), "first\nsecond");
  reference.model.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 5)),
    text: "saved",
  }]);

  await reference.save(textFiles, new AbortController().signal);
  assert.deepEqual(textFiles.savedTexts, ["saved\r\nsecond"]);
});

test("Alpha text model refuses to overwrite externally changed content", async () => {
  using models = new AlphaTextModelService();
  const textFiles = new TestTextFileService("from disk");
  const reference = await models.acquire({ resource: URI.file("C:\\project\\main.ts") }, textFiles, new AbortController().signal);
  reference.model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "local ",
  }]);
  textFiles.setText("external change");

  await assert.rejects(reference.save(textFiles, new AbortController().signal), error => error instanceof AlphaTextModelConflictError);
  assert.equal(reference.isDirty, true);
  assert.deepEqual(textFiles.savedTexts, []);
});

test("Alpha text model reloads clean external changes and marks dirty models conflicted", async () => {
  using models = new AlphaTextModelService();
  const resource = URI.file("C:\\project\\main.ts");
  const textFiles = new TestTextFileService("from disk");
  const reference = await models.acquire({ resource }, textFiles, new AbortController().signal);

  textFiles.setText("external clean");
  textFiles.fireExternalChange(resource);
  await waitFor(() => reference.model.getText() === "external clean");
  assert.equal(reference.isDirty, false);
  assert.equal(reference.hasExternalChange, false);

  const stableVersion = reference.model.version;
  textFiles.fireExternalChange(resource);
  await waitFor(() => textFiles.resolveCount === 3);
  assert.equal(reference.model.version, stableVersion);
  assert.equal(reference.hasExternalChange, false);

  reference.model.applyEdits([{
    range: TextRange.emptyAt(TextPosition.at(0, 0)),
    text: "local ",
  }]);
  textFiles.setText("external dirty");
  textFiles.fireExternalChange(resource);
  assert.equal(reference.hasExternalChange, true);
  await assert.rejects(reference.save(textFiles, new AbortController().signal), AlphaTextModelConflictError);
  await reference.revert(textFiles, new AbortController().signal);
  assert.equal(reference.model.getText(), "external dirty");
  assert.equal(reference.hasExternalChange, false);
});

class TestTextFileService implements ITextFileService {
  resolveCount = 0;
  readonly savedTexts: string[] = [];
  private readonly fileChanges = new Emitter<IFileChangeEvent>();
  readonly onDidChangeFiles = this.fileChanges.event;

  constructor(private text: string) {}

  async resolve(request: { resource: URI; bootstrapText?: string }) {
    this.resolveCount += 1;
    return {
      resource: request.resource,
      text: request.bootstrapText ?? this.text,
      source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
    };
  }

  async save(request: TextFileSaveRequest): Promise<void> {
    this.savedTexts.push(request.text);
    this.text = request.text;
  }

  setText(text: string): void {
    this.text = text;
  }

  fireExternalChange(resource: URI): void {
    this.fileChanges.fire(Object.freeze({ resources: Object.freeze([resource]) }));
  }
}

function inertFileChanges() {
  return {
    dispose() {},
    [Symbol.dispose]() {},
  };
}

async function waitFor(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 0));
  }
  assert.fail("Timed out waiting for Alpha external file synchronization");
}

function deferred<T>(): { readonly promise: Promise<T>; readonly resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(resolver => {
    resolve = resolver;
  });
  return { promise, resolve };
}

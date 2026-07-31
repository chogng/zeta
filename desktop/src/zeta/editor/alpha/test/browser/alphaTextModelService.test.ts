import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../../../../base/common/uri.js";
import { TextFileContentSource, type ITextFileService } from "../../../../workbench/services/textfile/common/textFileService.js";
import { AlphaTextModelService } from "../../browser/alphaTextModelService.js";
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

class TestTextFileService implements ITextFileService {
  resolveCount = 0;

  constructor(private readonly text: string) {}

  async resolve(request: { resource: URI; bootstrapText?: string }) {
    this.resolveCount += 1;
    return {
      resource: request.resource,
      text: request.bootstrapText ?? this.text,
      source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
    };
  }
}

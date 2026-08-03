import assert from "node:assert/strict";
import test from "node:test";
import { type DiffComputationRequest, type IDiffComputationService } from "../../../../common/models/diff/diffComputationService.js";
import { DiffModel } from "../../../../common/models/diff/diffModel.js";
import { InlineDiffComputationService } from "../../../../common/models/diff/inlineDiffComputationService.js";
import { computeLineDiff, type LineDiff } from "../../../../common/models/diff/lineDiff.js";
import { TextPosition, TextRange } from "../../../../common/text.js";
import { TextModel } from "../../../../common/textModel.js";

test("DiffModel publishes only version-pinned computation results", async () => {
  using original = new TextModel("before");
  using modified = new TextModel("after");
  using computationService = new ControlledDiffComputationService();
  using model = new DiffModel({ original, modified, computationService });

  assert.equal(model.state.kind, "loading");
  const first = computationService.takeRequest();
  original.applyEdits([{
    range: TextRange.from(TextPosition.at(0, 0), TextPosition.at(0, 6)),
    text: "current",
  }]);
  const second = computationService.takeRequest();
  assert.equal(first.signal.aborted, true);
  first.resolve(computeLineDiff(first.request.original.text, first.request.modified.text));
  await Promise.resolve();
  assert.equal(model.state.kind, "loading");

  second.resolve(computeLineDiff(second.request.original.text, second.request.modified.text));
  await waitForReady(model);
  const readyState = model.state;
  assert.equal(readyState.kind, "ready");
  if (readyState.kind !== "ready") throw new Error("Expected a ready diff model");
  assert.equal(readyState.originalVersion, original.version);
  assert.equal(readyState.modifiedVersion, modified.version);
  assert.equal(model.diff?.rows[0]?.kind, "modified");
});

test("DiffModel exposes an inline computation result without owning its sources", async () => {
  using original = new TextModel("same\nold");
  using modified = new TextModel("same\nnew");
  using computationService = new InlineDiffComputationService();
  using model = new DiffModel({ original, modified, computationService });

  await waitForReady(model);

  assert.equal(model.original, original);
  assert.equal(model.modified, modified);
  assert.equal(model.diff?.rows.length, 2);
  model.dispose();
  assert.equal(original.getText(), "same\nold");
  assert.equal(modified.getText(), "same\nnew");
});

interface ControlledRequest {
  readonly request: DiffComputationRequest;
  readonly signal: AbortSignal;
  readonly resolve: (diff: LineDiff) => void;
}

class ControlledDiffComputationService implements IDiffComputationService {
  private readonly requests: ControlledRequest[] = [];

  compute(request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
    return new Promise(resolve => this.requests.push({ request, signal, resolve }));
  }

  takeRequest(): ControlledRequest {
    const request = this.requests.shift();
    assert.ok(request);
    return request;
  }

  dispose(): void {}

  [Symbol.dispose](): void {
    this.dispose();
  }
}

function waitForReady(model: DiffModel): Promise<void> {
  if (model.state.kind === "ready") return Promise.resolve();
  return new Promise((resolve, reject) => {
    const listener = model.onDidChange(state => {
      if (state.kind === "loading") return;
      listener.dispose();
      if (state.kind === "error") reject(state.error);
      else resolve();
    });
  });
}

import assert from "node:assert/strict";
import test from "node:test";

import { URI } from "../../../../../base/common/uri.js";
import { TextPosition, TextRange } from "../../../../../editor/common/core/text.js";
import { type LanguageWorkspaceSymbol } from "../../../../../editor/common/languages/workspaceSymbols.js";
import { type IFileService } from "../../../../../platform/files/common/files.js";
import { type IEditorPart } from "../../../../browser/parts/editor/editorPart.js";
import { acceptWorkspaceSymbol } from "../../browser/workspaceSymbolNavigation.js";

const resource = URI.file("/workspace/src/main.rs");
const range = TextRange.from(TextPosition.at(2, 4), TextPosition.at(2, 8));

test("workspace symbol acceptance refreshes instead of opening a stale local result", async () => {
  const events = acceptanceEvents("current");

  await acceptWorkspaceSymbol(symbol("sha256:stale"), events.files, events.editor, events.quickPick, events.refresh);

  assert.equal(events.refreshed(), 1);
  assert.equal(events.hidden(), 0);
  assert.equal(events.opened(), 0);
});

test("workspace symbol acceptance opens a revision-verified local result", async () => {
  const events = acceptanceEvents("current");

  await acceptWorkspaceSymbol(symbol("sha256:current"), events.files, events.editor, events.quickPick, events.refresh);

  assert.equal(events.refreshed(), 0);
  assert.equal(events.hidden(), 1);
  assert.equal(events.opened(), 1);
});

function symbol(sourceRevision: string): LanguageWorkspaceSymbol {
  return { name: "main", kind: "function", resource, range, data: { source: "localSymbolIndex", sourceRevision } };
}

function acceptanceEvents(revision: string) {
  let hidden = 0;
  let refreshed = 0;
  let opened = 0;
  const files = { readFile: async () => ({ resource, content: "fn main() {}\n", revision }) } as unknown as IFileService;
  const editor = { openEditor: async () => { opened += 1; return {}; } } as unknown as IEditorPart;
  return {
    files,
    editor,
    quickPick: { hide: () => { hidden += 1; } },
    refresh: () => { refreshed += 1; },
    hidden: () => hidden,
    refreshed: () => refreshed,
    opened: () => opened,
  };
}

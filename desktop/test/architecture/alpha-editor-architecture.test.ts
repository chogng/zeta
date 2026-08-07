import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import test from "node:test";

const alphaRoot = resolve(import.meta.dirname, "../../../..", "src/zeta/editor/alpha");

test("Alpha keeps explicit feature files without index barrels", () => {
  const indexFiles = collectFiles(alphaRoot).filter(file => file.endsWith("\\index.ts") || file.endsWith("/index.ts"));
  assert.deepEqual(indexFiles, []);
});

test("Alpha synchronous layers do not import Workbench, Electron, or generated DTOs", () => {
  const protectedDirectories = [
    "common/core",
    "common/model",
    "common/cursor",
    "common/commands",
    "common/viewLayout",
    "common/viewModel",
  ];
  for (const directory of protectedDirectories) {
    for (const file of collectFiles(join(alphaRoot, directory))) {
      if (!file.endsWith(".ts")) continue;
      const source = readFileSync(file, "utf8");
      assert.doesNotMatch(source, /from\s+["'][^"']*(?:workbench|electron|generated)[^"']*["']/u, relative(alphaRoot, file));
    }
  }
});

test("Alpha implementation ledger names the active browser and service owners", () => {
  const requiredFiles = [
    "browser/editorPart.ts",
    "browser/browserEditorPart.ts",
    "browser/view/editorViewport.ts",
    "browser/input/textInputController.ts",
    "browser/services/rustDiffComputationService.ts",
    "common/core/position.ts",
    "common/model/decorationCollection.ts",
    "common/model/textModel.ts",
    "common/cursor/editorSelectionController.ts",
    "common/services/languageService.ts",
    "contrib/gotoError/browser/gotoError.ts",
    "contrib/indentation/browser/indentation.ts",
    "contrib/gpu/browser/gpuRenderer.ts",
    "contrib/longLinesHelper/browser/longLinesHelper.ts",
    "contrib/tokenization/common/tokenizationTextModelPart.ts",
    "contrib/semanticTokens/common/semanticTokens.ts",
    "alpha-implementation-ledger.md",
  ];
  for (const file of requiredFiles) assert.equal(statSafe(join(alphaRoot, file)), true, file);

  const removedLegacyNames = [
    "browser/editorSession.ts",
    "browser/browserEditorSession.ts",
    "common/model/decoration.ts",
    "contrib/gotoError/browser/gotoErrorController.ts",
    "contrib/indentation/browser/indentationGuides.ts",
  ];
  for (const file of removedLegacyNames) assert.equal(statSafe(join(alphaRoot, file)), false, file);
});

test("Alpha PieceTree tests follow VS Code's common model layout", () => {
  assert.equal(statSafe(join(alphaRoot, "test/common/model/pieceTreeTextBuffer/pieceTreeTextBuffer.test.ts")), true);
  assert.equal(statSafe(join(alphaRoot, "test/common/pieceTreeTextBuffer.test.ts")), false);
});

function collectFiles(directory: string): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const file = join(directory, entry.name);
    if (entry.isDirectory()) result.push(...collectFiles(file));
    else result.push(file);
  }
  return result;
}

function statSafe(file: string): boolean {
  try {
    return statSync(file).isFile();
  } catch {
    return false;
  }
}

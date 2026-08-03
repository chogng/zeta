import assert from "node:assert/strict";
import test from "node:test";
import { AlphaLineDiffKind, computeAlphaLineDiff } from "../../common/lineDiff.js";

test("Alpha line diff aligns equal, modified, removed, and added rows", () => {
  const diff = computeAlphaLineDiff(
    "same\nold value\nremoved\ntail",
    "same\nnew value\nadded\ntail",
  );

  assert.equal(diff.approximate, false);
  assert.deepEqual(diff.rows, [
    row(AlphaLineDiffKind.Unchanged, 0, 0),
    row(AlphaLineDiffKind.Modified, 1, 1, [{ startColumn: 0, endColumn: 3 }], [{ startColumn: 0, endColumn: 3 }]),
    row(AlphaLineDiffKind.Modified, 2, 2, [{ startColumn: 0, endColumn: 5 }], [{ startColumn: 0, endColumn: 3 }]),
    row(AlphaLineDiffKind.Unchanged, 3, 3),
  ]);
});

test("Alpha line diff preserves moved insertions and deletions as aligned gaps", () => {
  const diff = computeAlphaLineDiff("one\nthree", "one\ntwo\nthree");
  assert.deepEqual(diff.rows, [
    row(AlphaLineDiffKind.Unchanged, 0, 0),
    row(AlphaLineDiffKind.Added, undefined, 1),
    row(AlphaLineDiffKind.Unchanged, 1, 2),
  ]);
});

test("Alpha line diff marks surrogate-pair changes without splitting a grapheme", () => {
  const diff = computeAlphaLineDiff("before 🙂 after", "before 🙃 after");
  assert.deepEqual(diff.rows, [
    row(AlphaLineDiffKind.Modified, 0, 0, [{ startColumn: 7, endColumn: 9 }], [{ startColumn: 7, endColumn: 9 }]),
  ]);
});

test("Alpha line diff uses a conservative bounded fallback", () => {
  const diff = computeAlphaLineDiff("a\nb\nc", "x\ny\nz", { maximumComputationSteps: 1 });
  assert.equal(diff.approximate, true);
  assert.deepEqual(diff.rows.map(row => row.kind), [
    AlphaLineDiffKind.Modified,
    AlphaLineDiffKind.Modified,
    AlphaLineDiffKind.Modified,
  ]);
});

test("Alpha line diff preserves every source line and never marks unequal lines unchanged", () => {
  const random = seededRandom(0xD1FF);
  for (let iteration = 0; iteration < 500; iteration += 1) {
    const originalLines = randomLines(random);
    const modifiedLines = randomLines(random);
    const diff = computeAlphaLineDiff(originalLines.join("\n"), modifiedLines.join("\n"), { maximumComputationSteps: 100_000 });
    assert.equal(diff.approximate, false, `unexpected bounded fallback at ${iteration}`);
    assert.deepEqual(diff.rows.flatMap(row => row.originalLineIndex === undefined ? [] : [row.originalLineIndex]), Array.from({ length: originalLines.length }, (_, index) => index));
    assert.deepEqual(diff.rows.flatMap(row => row.modifiedLineIndex === undefined ? [] : [row.modifiedLineIndex]), Array.from({ length: modifiedLines.length }, (_, index) => index));
    for (const row of diff.rows) {
      if (row.kind !== AlphaLineDiffKind.Unchanged) continue;
      assert.equal(originalLines[row.originalLineIndex!], modifiedLines[row.modifiedLineIndex!], `unequal lines were marked unchanged at ${iteration}`);
    }
  }
});

function row(kind: AlphaLineDiffKind, originalLineIndex?: number, modifiedLineIndex?: number, originalChanges: readonly unknown[] = [], modifiedChanges: readonly unknown[] = []) {
  return {
    kind,
    ...(originalLineIndex === undefined ? {} : { originalLineIndex }),
    ...(modifiedLineIndex === undefined ? {} : { modifiedLineIndex }),
    originalChanges,
    modifiedChanges,
  };
}

function randomLines(random: () => number): string[] {
  const values = ["", "a", "b", "same", "🙂", "alpha"];
  return Array.from({ length: Math.floor(random() * 12) + 1 }, () => values[Math.floor(random() * values.length)]!);
}

function seededRandom(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
    return state / 0x1_0000_0000;
  };
}

import { performance } from "node:perf_hooks";
import { PieceTreeTextBuffer } from "../common/pieceTreeTextBuffer.js";

interface BenchmarkResult {
  readonly workload: string;
  readonly milliseconds: number;
}

const results: BenchmarkResult[] = [];
const initialText = "const value = 1234567890;\n".repeat(80_000);
let buffer = new PieceTreeTextBuffer("");

measure("construct 2 MiB document", () => {
  buffer = new PieceTreeTextBuffer(initialText);
});

measure("10k scattered replacements", () => {
  for (let index = 0; index < 10_000; index += 1) {
    const offset = (
      (Math.imul(index + 1, 104_729) >>> 0) %
      buffer.length
    );
    buffer.replace(offset, offset + 1, String.fromCharCode(
      97 + index % 26,
    ));
    buffer.compactIfNeeded();
  }
});

measure("100k coordinate round trips", () => {
  for (let index = 0; index < 100_000; index += 1) {
    const offset = (
      (Math.imul(index + 1, 65_537) >>> 0) %
      (buffer.length + 1)
    );
    const position = buffer.positionAt(offset);
    buffer.offsetAt(position.lineIndex, position.columnIndex);
  }
});

measure("snapshot and 10k range reads", () => {
  const snapshot = buffer.createSnapshot();
  for (let index = 0; index < 10_000; index += 1) {
    const startOffset = (
      (Math.imul(index + 1, 8_191) >>> 0) %
      (snapshot.length - 128)
    );
    snapshot.getTextBetweenOffsets(startOffset, startOffset + 128);
  }
});

const churnBuffer = new PieceTreeTextBuffer("");
const churnText = "0123456789abcdef".repeat(128 * 1_024);
churnBuffer.replace(0, 0, churnText);
churnBuffer.replace(0, churnText.length - 64 * 1_024, "");
const retainedBeforeCompaction =
  churnBuffer.getStatistics().retainedTextUnits;

measure("compact 2 MiB churn buffer", () => {
  churnBuffer.compactIfNeeded();
});

console.table(results);
console.log({
  editedDocument: buffer.getStatistics(),
  churnRetainedBeforeCompaction: retainedBeforeCompaction,
  churnAfterCompaction: churnBuffer.getStatistics(),
});

function measure(workload: string, run: () => void): void {
  const start = performance.now();
  run();
  results.push({
    workload,
    milliseconds: Math.round((performance.now() - start) * 100) / 100,
  });
}

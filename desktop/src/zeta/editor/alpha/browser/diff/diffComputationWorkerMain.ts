import { computeLineDiff } from "../../common/models/diff/lineDiff.js";
import { type DiffComputationRequest } from "../../common/models/diff/diffComputationService.js";

interface ComputeMessage {
  readonly kind: "compute";
  readonly requestId: number;
  readonly request: DiffComputationRequest;
}

interface DedicatedWorkerScope {
  postMessage(message: unknown): void;
  addEventListener(type: "message", listener: (event: { readonly data: unknown }) => void): void;
}

const scope = globalThis as unknown as DedicatedWorkerScope;

scope.addEventListener("message", event => {
  const message = event.data;
  if (!isComputeMessage(message)) return;
  try {
    scope.postMessage(Object.freeze({
      kind: "result",
      requestId: message.requestId,
      diff: computeLineDiff(
        message.request.original.text,
        message.request.modified.text,
        message.request.options,
      ),
    }));
  } catch (error) {
    const failure = asError(error);
    scope.postMessage(Object.freeze({
      kind: "failure",
      requestId: message.requestId,
      error: Object.freeze({ name: failure.name, message: failure.message }),
    }));
  }
});

function isComputeMessage(value: unknown): value is ComputeMessage {
  return isRecord(value) &&
    value.kind === "compute" &&
    Number.isSafeInteger(value.requestId) &&
    value.requestId > 0 &&
    isRecord(value.request) &&
    isDocument(value.request.original) &&
    isDocument(value.request.modified) &&
    isRecord(value.request.options);
}

function isDocument(value: unknown): value is DiffComputationRequest["original"] {
  return isRecord(value) && Number.isSafeInteger(value.version) && value.version > 0 && typeof value.text === "string";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}

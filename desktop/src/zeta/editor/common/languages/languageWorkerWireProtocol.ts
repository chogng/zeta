import { type LanguageWorkerRequest } from "./languageRequestCoordinator.js";
import { normalizeTextLineEndings, type TextModelChange, type TextSnapshot } from "../core/text.js";

const LANGUAGE_WORKER_PROTOCOL = "zeta.language-worker";
const LANGUAGE_WORKER_PROTOCOL_VERSION = 4;

export interface LanguageWorkerWireResultState<TResult> {
  readonly requestId: number;
  readonly snapshot: TextSnapshot;
  readonly result: TResult;
}

export type LanguageWorkerWireResultProtocol = "stateless" | "confirmedBase";

export interface LanguageWorkerWireCodec<TLane extends string, TPayload, TResult> {
  readonly lanes: readonly TLane[];
  readonly resultProtocol: LanguageWorkerWireResultProtocol;
  encodePayload(lane: TLane, payload: TPayload): unknown;
  decodePayload(lane: TLane, value: unknown, snapshot: TextSnapshot): TPayload;
  encodeResult(lane: TLane, result: TResult, snapshot: TextSnapshot, base: LanguageWorkerWireResultState<TResult> | undefined): unknown;
  decodeResult(lane: TLane, value: unknown, snapshot: TextSnapshot, base: LanguageWorkerWireResultState<TResult> | undefined): TResult;
}

export interface EncodedRequestWireMessage {
  readonly message: RequestWireMessage;
  readonly establishesMirrorVersion: number | undefined;
}

export interface DecodedRequestSnapshot {
  readonly snapshot: TextSnapshot;
  readonly replacesMirror: boolean;
}

export interface RequestWireMessage {
  readonly protocol: typeof LANGUAGE_WORKER_PROTOCOL;
  readonly version: typeof LANGUAGE_WORKER_PROTOCOL_VERSION;
  readonly kind: "request";
  readonly requestId: number;
  readonly lane: string;
  readonly resultBaseRequestId?: number;
  readonly snapshot: FullSnapshotWireDto | ReferencedSnapshotWireDto;
  readonly payload: unknown;
}

export interface CancelWireMessage {
  readonly protocol: typeof LANGUAGE_WORKER_PROTOCOL;
  readonly version: typeof LANGUAGE_WORKER_PROTOCOL_VERSION;
  readonly kind: "cancel";
  readonly requestId: number;
}

export interface SyncWireMessage {
  readonly protocol: typeof LANGUAGE_WORKER_PROTOCOL;
  readonly version: typeof LANGUAGE_WORKER_PROTOCOL_VERSION;
  readonly kind: "sync";
  readonly previousVersion: number;
  readonly modelVersion: number;
  readonly changes: readonly ContentChangeWireDto[];
}

export interface ResultWireMessage {
  readonly protocol: typeof LANGUAGE_WORKER_PROTOCOL;
  readonly version: typeof LANGUAGE_WORKER_PROTOCOL_VERSION;
  readonly kind: "result";
  readonly requestId: number;
  readonly result: unknown;
}

export interface FailureWireMessage {
  readonly protocol: typeof LANGUAGE_WORKER_PROTOCOL;
  readonly version: typeof LANGUAGE_WORKER_PROTOCOL_VERSION;
  readonly kind: "failure";
  readonly requestId: number;
  readonly error: ErrorWireDto;
}

export interface SyncFailureWireMessage {
  readonly protocol: typeof LANGUAGE_WORKER_PROTOCOL;
  readonly version: typeof LANGUAGE_WORKER_PROTOCOL_VERSION;
  readonly kind: "syncFailure";
  readonly error: ErrorWireDto;
}

interface FullSnapshotWireDto {
  readonly kind: "full";
  readonly version: number;
  readonly length: number;
  readonly lineCount: number;
  readonly text: string;
}

interface ReferencedSnapshotWireDto {
  readonly kind: "reference";
  readonly version: number;
  readonly length: number;
  readonly lineCount: number;
}

interface ContentChangeWireDto {
  readonly rangeOffset: number;
  readonly rangeLength: number;
  readonly text: string;
}

interface ErrorWireDto {
  readonly name: string;
  readonly message: string;
}

export function encodeRequestMessage<TLane extends string, TPayload, TResult>(request: LanguageWorkerRequest<TLane, TPayload>, codec: LanguageWorkerWireCodec<TLane, TPayload, TResult>, mirroredVersion: number | undefined, resultBaseRequestId: number | undefined): EncodedRequestWireMessage {
  if (resultBaseRequestId !== undefined) assertRequestId(resultBaseRequestId);
  const snapshot = mirroredVersion === request.snapshot.version
    ? encodeReferencedSnapshot(request.snapshot)
    : encodeFullSnapshot(request.snapshot);
  return Object.freeze({
    message: Object.freeze<RequestWireMessage>({
      protocol: LANGUAGE_WORKER_PROTOCOL,
      version: LANGUAGE_WORKER_PROTOCOL_VERSION,
      kind: "request",
      requestId: request.requestId,
      lane: request.lane,
      ...(resultBaseRequestId === undefined ? {} : { resultBaseRequestId }),
      snapshot,
      payload: codec.encodePayload(request.lane, request.payload),
    }),
    establishesMirrorVersion: snapshot.kind === "full" ? snapshot.version : undefined,
  });
}

export function encodeSyncMessage(change: TextModelChange): SyncWireMessage {
  assertPositiveSafeInteger(change.version, "Language worker sync model version");
  if (change.version <= 1 || !Array.isArray(change.changes) || change.changes.length === 0) {
    throw new RangeError("Language worker sync must describe one committed model version");
  }
  return Object.freeze({
    protocol: LANGUAGE_WORKER_PROTOCOL,
    version: LANGUAGE_WORKER_PROTOCOL_VERSION,
    kind: "sync",
    previousVersion: change.version - 1,
    modelVersion: change.version,
    changes: Object.freeze(change.changes.map(change => {
      assertNonNegativeSafeInteger(change.rangeOffset, "Language worker sync range offset");
      assertNonNegativeSafeInteger(change.rangeLength, "Language worker sync range length");
      if (typeof change.text !== "string" || normalizeTextLineEndings(change.text) !== change.text) {
        throw new TypeError("Language worker sync text must use normalized LF line endings");
      }
      return Object.freeze({
        rangeOffset: change.rangeOffset,
        rangeLength: change.rangeLength,
        text: change.text,
      });
    })),
  });
}

export function decodeRequestSnapshot(value: RequestWireMessage["snapshot"], mirror: TextSnapshot | undefined): DecodedRequestSnapshot {
  assertRecord(value, "Language worker snapshot");
  if (value.kind === "full") {
    return Object.freeze({
      snapshot: decodeFullSnapshot(value as FullSnapshotWireDto),
      replacesMirror: true,
    });
  }
  if (value.kind !== "reference") {
    throw new TypeError(`Unknown language worker snapshot kind '${String(value.kind)}'`);
  }
  assertSnapshotMetadata(value);
  if (!mirror) {
    throw new Error("Language worker request references an unavailable snapshot mirror");
  }
  if (mirror.version !== value.version || mirror.length !== value.length || mirror.lineCount !== value.lineCount) {
    throw new Error("Language worker request snapshot reference does not match its mirror");
  }
  return Object.freeze({ snapshot: mirror, replacesMirror: false });
}

export function createCancelMessage(requestId: number): CancelWireMessage {
  return Object.freeze({ protocol: LANGUAGE_WORKER_PROTOCOL, version: LANGUAGE_WORKER_PROTOCOL_VERSION, kind: "cancel", requestId });
}

export function createResultMessage(requestId: number, result: unknown): ResultWireMessage {
  return Object.freeze({ protocol: LANGUAGE_WORKER_PROTOCOL, version: LANGUAGE_WORKER_PROTOCOL_VERSION, kind: "result", requestId, result });
}

export function createFailureMessage(requestId: number, error: unknown): FailureWireMessage {
  return Object.freeze({
    protocol: LANGUAGE_WORKER_PROTOCOL,
    version: LANGUAGE_WORKER_PROTOCOL_VERSION,
    kind: "failure",
    requestId,
    error: encodeError(error, "Remote language worker failed"),
  });
}

export function createSyncFailureMessage(error: unknown): SyncFailureWireMessage {
  return Object.freeze({
    protocol: LANGUAGE_WORKER_PROTOCOL,
    version: LANGUAGE_WORKER_PROTOCOL_VERSION,
    kind: "syncFailure",
    error: encodeError(error, "Remote language worker synchronization failed"),
  });
}

export function decodeClientMessage(value: Record<string, unknown>): ResultWireMessage | FailureWireMessage | SyncFailureWireMessage {
  assertEnvelope(value);
  if (value.kind === "syncFailure") {
    assertError(value.error);
    return value as unknown as SyncFailureWireMessage;
  }
  assertRequestId(value.requestId);
  if (value.kind === "result") return value as unknown as ResultWireMessage;
  if (value.kind === "failure") {
    assertError(value.error);
    return value as unknown as FailureWireMessage;
  }
  throw new TypeError(`Unknown language worker client message '${String(value.kind)}'`);
}

export function decodeServerMessage(value: Record<string, unknown>): RequestWireMessage | CancelWireMessage | SyncWireMessage {
  assertEnvelope(value);
  if (value.kind === "sync") {
    assertPositiveSafeInteger(value.previousVersion, "Language worker sync previous version");
    assertPositiveSafeInteger(value.modelVersion, "Language worker sync model version");
    if (!Array.isArray(value.changes)) {
      throw new TypeError("Language worker sync changes must be an array");
    }
    return value as unknown as SyncWireMessage;
  }
  assertRequestId(value.requestId);
  if (value.kind === "cancel") return value as unknown as CancelWireMessage;
  if (value.kind === "request") {
    if (typeof value.lane !== "string" || value.lane.length === 0) {
      throw new TypeError("Language worker request lane must be a non-empty string");
    }
    if (value.resultBaseRequestId !== undefined) {
      assertRequestId(value.resultBaseRequestId);
    }
    assertRecord(value.snapshot, "Language worker snapshot");
    return value as unknown as RequestWireMessage;
  }
  throw new TypeError(`Unknown language worker server message '${String(value.kind)}'`);
}

export function isProtocolMessage(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && (value as Record<string, unknown>).protocol === LANGUAGE_WORKER_PROTOCOL;
}

export function readRequestId(value: Record<string, unknown>): number | undefined {
  return Number.isSafeInteger(value.requestId) && (value.requestId as number) > 0 ? value.requestId as number : undefined;
}

export function assertRequestId(value: unknown): asserts value is number {
  assertPositiveSafeInteger(value, "Language worker request ID");
}

function encodeFullSnapshot(snapshot: TextSnapshot): FullSnapshotWireDto {
  const value = Object.freeze({
    kind: "full" as const,
    version: snapshot.version,
    length: snapshot.length,
    lineCount: snapshot.lineCount,
    text: snapshot.getText(),
  });
  assertFullSnapshot(value);
  return value;
}

function encodeReferencedSnapshot(snapshot: TextSnapshot): ReferencedSnapshotWireDto {
  const value = Object.freeze({
    kind: "reference" as const,
    version: snapshot.version,
    length: snapshot.length,
    lineCount: snapshot.lineCount,
  });
  assertSnapshotMetadata(value);
  return value;
}

function decodeFullSnapshot(value: FullSnapshotWireDto): TextSnapshot {
  assertFullSnapshot(value);
  return createSnapshot(value.version, value.text, value.lineCount);
}

function assertFullSnapshot(value: FullSnapshotWireDto): void {
  assertSnapshotMetadata(value);
  if (typeof value.text !== "string" || normalizeTextLineEndings(value.text) !== value.text) {
    throw new TypeError("Language worker snapshot text must use normalized LF line endings");
  }
  if (value.length !== value.text.length) {
    throw new RangeError("Language worker snapshot length does not match its text");
  }
  if (value.lineCount !== countLines(value.text)) {
    throw new RangeError("Language worker snapshot line count does not match its text");
  }
}

function createSnapshot(version: number, text: string, lineCount = countLines(text)): TextSnapshot {
  return Object.freeze({
    version,
    length: text.length,
    lineCount,
    getText: () => text,
    getTextBetweenOffsets: (startOffset: number, endOffset: number) => {
      assertNonNegativeSafeInteger(startOffset, "Snapshot start offset");
      assertNonNegativeSafeInteger(endOffset, "Snapshot end offset");
      if (startOffset > endOffset || endOffset > text.length) {
        throw new RangeError("Snapshot offset range is outside its text");
      }
      return text.slice(startOffset, endOffset);
    },
  });
}

function assertSnapshotMetadata(value: { readonly version: unknown; readonly length: unknown; readonly lineCount: unknown }): void {
  assertPositiveSafeInteger(value.version, "Language worker snapshot version");
  assertNonNegativeSafeInteger(value.length, "Language worker snapshot length");
  assertPositiveSafeInteger(value.lineCount, "Language worker snapshot line count");
}

function assertEnvelope(value: Record<string, unknown>): void {
  if (value.version !== LANGUAGE_WORKER_PROTOCOL_VERSION) {
    throw new RangeError(`Unsupported language worker protocol version '${String(value.version)}'`);
  }
}

function assertError(value: unknown): asserts value is ErrorWireDto {
  assertRecord(value, "Language worker failure");
  if (typeof value.name !== "string" || typeof value.message !== "string") {
    throw new TypeError("Language worker failure must contain string name and message");
  }
}

function encodeError(value: unknown, fallbackMessage: string): ErrorWireDto {
  const error = value instanceof Error ? value : new Error(value === undefined ? fallbackMessage : String(value));
  return Object.freeze({ name: error.name, message: error.message });
}

function assertPositiveSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) <= 0) {
    throw new RangeError(`${owner} must be a positive safe integer`);
  }
}

function assertNonNegativeSafeInteger(value: unknown, owner: string): asserts value is number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new RangeError(`${owner} must be a non-negative safe integer`);
  }
}

function assertRecord(value: unknown, owner: string): asserts value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${owner} must be an object`);
  }
}

function countLines(text: string): number {
  let result = 1;
  for (let index = 0; index < text.length; index += 1) {
    if (text.charCodeAt(index) === 10) result += 1;
  }
  return result;
}

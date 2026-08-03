import { decodeLanguageAnalysisWireResult, encodeLanguageAnalysisWireResult } from "./languageAnalysisWireResult.js";
import { assertLanguageAnalysisRequest, type LanguageAnalysisRequest } from "./languageAnalysisProviders.js";
import { LANGUAGE_DIAGNOSTIC_LANE, LANGUAGE_TOKEN_LANE, type LanguageAnalysisLane, type LanguageAnalysisResult } from "./languageAnalysisService.js";
import { type LanguageWorkerWireCodec } from "./languageWorkerWire.js";
import { type TextSnapshot } from "../../common/text.js";

export const languageAnalysisWireCodec: LanguageWorkerWireCodec<LanguageAnalysisLane, LanguageAnalysisRequest, LanguageAnalysisResult> = Object.freeze({
  lanes: Object.freeze([LANGUAGE_TOKEN_LANE, LANGUAGE_DIAGNOSTIC_LANE] as const),
  resultProtocol: "confirmedBase",
  encodePayload(_lane: LanguageAnalysisLane, request: LanguageAnalysisRequest) {
    assertLanguageAnalysisRequest(request);
    return Object.freeze({ languageId: request.languageId });
  },
  decodePayload(_lane: LanguageAnalysisLane, value: unknown, _snapshot: TextSnapshot) {
    assertRecord(value, "Language analysis wire request");
    const request = Object.freeze({
      languageId: decodeString(value.languageId, "Language analysis wire language ID"),
    });
    assertLanguageAnalysisRequest(request);
    return request;
  },
  encodeResult: encodeLanguageAnalysisWireResult,
  decodeResult: decodeLanguageAnalysisWireResult,
});

function decodeString(value: unknown, owner: string): string {
  if (typeof value !== "string") throw new TypeError(`${owner} must be a string`);
  return value;
}

function assertRecord(value: unknown, owner: string): asserts value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new TypeError(`${owner} must be an object`);
}

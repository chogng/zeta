/**
 * Stable text primitive barrel for Stanza consumers.
 *
 * The implementations live in focused modules so coordinate values, edit
 * descriptions, and committed model changes have separate ownership while
 * callers retain one convenient DOM-free text contract.
 */
export { TextPosition } from "./position.js";
export type { IPosition } from "./position.js";
export { TextRange } from "./range.js";
export type { IRange, ITextRange } from "./range.js";
export { EditOperation, TextEditHistoryGroup, TextEditHistoryMergeMode } from "./editOperation.js";
export type { ISingleEditOperation, TextEdit } from "./editOperation.js";
export { TextChange, compressConsecutiveTextChanges, normalizeTextLineEndings, TextModelChangeReason } from "./textChange.js";
export type { TextModelChange, TextModelContentChange, TextSnapshot } from "./textChange.js";
export { AbstractText, ArrayText, LineBasedText, StringText } from "./text/abstractText.js";
export { LineBasedPositionOffsetTransformer, PositionOffsetTransformer, PositionOffsetTransformerBase } from "./text/positionToOffsetImpl.js";
export { getPositionOffsetTransformerFromTextModel } from "./text/getPositionOffsetTransformerFromTextModel.js";
export type { TextModelLineSource } from "./text/getPositionOffsetTransformerFromTextModel.js";
export { TextLength } from "./text/textLength.js";

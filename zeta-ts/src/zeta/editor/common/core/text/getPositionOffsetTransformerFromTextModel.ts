import { LineBasedPositionOffsetTransformer } from "./positionToOffsetImpl.js";

/** Minimal model surface needed by the core coordinate adapter. */
export interface TextModelLineSource {
	readonly lineCount: number;
	getLineContent(lineIndex: number): string;
}

/**
 * Creates a detached transformer from a model's current logical lines.
 *
 * The core only depends on this structural contract; importing `TextModel`
 * here would reverse the dependency from common/core into the mutable model.
 */
export function getPositionOffsetTransformerFromTextModel(model: TextModelLineSource): LineBasedPositionOffsetTransformer {
	const lines = Array.from({ length: model.lineCount }, (_, lineIndex) => model.getLineContent(lineIndex));
	return new LineBasedPositionOffsetTransformer(lines);
}

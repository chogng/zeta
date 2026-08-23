import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** Returns non-LF line-separator characters that survived model normalization. */
export function findUnusualLineTerminators(model: TextModel): readonly TextRange[] {
	const result: TextRange[] = [];
	for (let lineIndex = 0; lineIndex < model.lineCount; lineIndex += 1) {
		const line = model.getLineContent(lineIndex);
		let columnIndex = 0;
		for (const character of line) {
			const codePoint = character.codePointAt(0)!;
			if (codePoint === 0x2028 || codePoint === 0x2029 || codePoint === 0x0085) result.push(TextRange.from(TextPosition.at(lineIndex, columnIndex), TextPosition.at(lineIndex, columnIndex + character.length)));
			columnIndex += character.length;
		}
	}
	return Object.freeze(result);
}

import { CursorColumns } from "../cursorColumns.js";

export function normalizeIndentation(value: string, indentSize: number, insertSpaces: boolean): string {
	if (!Number.isSafeInteger(indentSize) || indentSize <= 0) throw new RangeError("Indent size must be a positive safe integer");
	let columns = 0;
	let firstNonWhitespace = 0;
	while (firstNonWhitespace < value.length && (value[firstNonWhitespace] === " " || value[firstNonWhitespace] === "\t")) firstNonWhitespace += 1;
	for (let index = 0; index < firstNonWhitespace; index += 1) {
		const character = value[index];
		if (character === "\t") columns = CursorColumns.nextIndentTabStop(columns, indentSize);
		else columns += 1;
	}
	const normalized = insertSpaces
		? " ".repeat(columns)
		: "\t".repeat(Math.floor(columns / indentSize)) + " ".repeat(columns % indentSize);
	return normalized + value.slice(firstNonWhitespace);
}

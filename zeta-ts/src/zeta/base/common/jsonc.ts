import { parseJsonDocument } from './json.js';

/** Parses JSON with line comments, block comments, and trailing commas. */
export function parseJsonc(source: string, owner: string): unknown {
	if (typeof source !== 'string') throw new TypeError(`${owner} must be text`);
	const document = parseJsonDocument(source, { allowComments: true, allowTrailingComma: true });
	const error = document.errors[0];
	if (error) throw new TypeError(`${owner} is not valid JSONC at offset ${error.offset}: ${error.message}`);
	return document.value;
}

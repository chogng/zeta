/** Escapes every regular-expression metacharacter in a literal string. */
export function escapeRegExpCharacters(value: string): string {
	return value.replace(/[\\^$.*+?()[\]{}|]/gu, '\\$&');
}

/** Returns the number of equal UTF-16 code units at the beginning of two strings. */
export function commonPrefixLength(left: string, right: string): number {
	const limit = Math.min(left.length, right.length);
	let index = 0;
	while (index < limit && left.charCodeAt(index) === right.charCodeAt(index)) index += 1;
	return index;
}

/** Returns the number of equal UTF-16 code units at the end of two strings. */
export function commonSuffixLength(left: string, right: string): number {
	const limit = Math.min(left.length, right.length);
	let length = 0;
	while (
		length < limit &&
		left.charCodeAt(left.length - length - 1) === right.charCodeAt(right.length - length - 1)
	) {
		length += 1;
	}
	return length;
}

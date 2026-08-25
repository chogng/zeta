/** Compares two array-like sequences item by item. */
export function arraysEqual<T>(
	left: readonly T[],
	right: readonly T[],
	equals: (left: T, right: T) => boolean = strictEquals,
): boolean {
	if (left === right) return true;
	if (left.length !== right.length) return false;
	return left.every((value, index) => equals(value, right[index]!));
}

/** Returns whether the provided array has at least one element. */
export function isNonEmptyArray<T>(obj: T[] | undefined | null): obj is T[];
export function isNonEmptyArray<T>(obj: readonly T[] | undefined | null): obj is readonly T[];
export function isNonEmptyArray<T>(obj: T[] | readonly T[] | undefined | null): obj is T[] | readonly T[] {
	return Array.isArray(obj) && obj.length > 0;
}

/** Returns the number of equal items at the beginning of two sequences. */
export function commonPrefixLength<T>(
	left: readonly T[],
	right: readonly T[],
	equals: (left: T, right: T) => boolean = strictEquals,
): number {
	const limit = Math.min(left.length, right.length);
	let index = 0;
	while (index < limit && equals(left[index]!, right[index]!)) index += 1;
	return index;
}

/** Returns the number of equal trailing items without overlapping an already matched prefix. */
export function commonSuffixLength<T>(
	left: readonly T[],
	right: readonly T[],
	prefixLength = 0,
	equals: (left: T, right: T) => boolean = strictEquals,
): number {
	const limit = Math.min(left.length, right.length);
	if (!Number.isInteger(prefixLength) || prefixLength < 0 || prefixLength > limit) {
		throw new RangeError('Prefix length must be an integer within both sequences');
	}
	const maximumLength = limit - prefixLength;
	let length = 0;
	while (
		length < maximumLength &&
		equals(left[left.length - length - 1]!, right[right.length - length - 1]!)
	) {
		length += 1;
	}
	return length;
}

function strictEquals<T>(left: T, right: T): boolean {
	return left === right;
}

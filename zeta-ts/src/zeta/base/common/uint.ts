export const enum Constants {
	MAX_SAFE_SMALL_INTEGER = 1 << 30,
	MAX_UINT_32 = 0xffffffff,
}

export function toUint8(value: number): number {
	return Math.min(255, Math.max(0, value | 0));
}

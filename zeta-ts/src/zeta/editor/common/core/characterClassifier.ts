/** A compact character classifier with a fast ASCII path and sparse Unicode map. */
export class CharacterClassifier<T extends number> {
	private readonly asciiMap: Uint8Array;
	private readonly map = new Map<number, number>();
	private readonly defaultValue: number;

	constructor(defaultValue: T) {
		this.defaultValue = toUint8(defaultValue);
		this.asciiMap = new Uint8Array(256);
		this.asciiMap.fill(this.defaultValue);
	}

	set(charCode: number, value: T): void {
		if (!Number.isSafeInteger(charCode) || charCode < 0) throw new RangeError("Character code must be a non-negative safe integer");
		const normalizedValue = toUint8(value);
		if (charCode < 256) this.asciiMap[charCode] = normalizedValue;
		else this.map.set(charCode, normalizedValue);
	}

	get(charCode: number): T {
		const value = charCode >= 0 && charCode < 256 ? this.asciiMap[charCode] : this.map.get(charCode) ?? this.defaultValue;
		return value as T;
	}

	clear(): void {
		this.asciiMap.fill(this.defaultValue);
		this.map.clear();
	}
}

export class CharacterSet {
	private readonly actual = new CharacterClassifier<CharacterSetValue>(CharacterSetValue.False);

	add(charCode: number): void { this.actual.set(charCode, CharacterSetValue.True); }
	has(charCode: number): boolean { return this.actual.get(charCode) === CharacterSetValue.True; }
	clear(): void { this.actual.clear(); }
}

enum CharacterSetValue { False = 0, True = 1 }

function toUint8(value: number): number {
	if (!Number.isSafeInteger(value) || value < 0 || value > 255) throw new RangeError("Character classifier values must fit in one byte");
	return value;
}

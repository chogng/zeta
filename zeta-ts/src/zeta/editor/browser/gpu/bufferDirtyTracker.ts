export interface IBufferDirtyTrackerReader {
	readonly dataOffset: number | undefined;
	readonly dirtySize: number | undefined;
	readonly isDirty: boolean;
	clear(): void;
}

export class BufferDirtyTracker implements IBufferDirtyTrackerReader {
	private startIndex: number | undefined;
	private endIndex: number | undefined;

	public get dataOffset(): number | undefined {
		return this.startIndex;
	}

	public get dirtySize(): number | undefined {
		if (this.startIndex === undefined || this.endIndex === undefined) return undefined;
		return this.endIndex - this.startIndex + 1;
	}

	public get isDirty(): boolean {
		return this.startIndex !== undefined;
	}

	public flag(index: number, length = 1): number {
		if (!Number.isSafeInteger(index) || index < 0) throw new RangeError('A dirty buffer index must be a non-negative integer');
		if (!Number.isSafeInteger(length) || length < 1) throw new RangeError('A dirty buffer length must be a positive integer');
		this.flagIndex(index);
		this.flagIndex(index + length - 1);
		return index;
	}

	public clear(): void {
		this.startIndex = undefined;
		this.endIndex = undefined;
	}

	private flagIndex(index: number): void {
		if (this.startIndex === undefined || index < this.startIndex) this.startIndex = index;
		if (this.endIndex === undefined || index > this.endIndex) this.endIndex = index;
	}
}

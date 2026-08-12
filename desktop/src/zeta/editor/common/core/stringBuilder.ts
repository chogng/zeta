/** Decodes UTF-16LE code units without dropping a leading BOM. */
let platformTextDecoder: TextDecoder | undefined;

export function getPlatformTextDecoder(): TextDecoder {
  return platformTextDecoder ??= new TextDecoder();
}

export function decodeUTF16LE(source: Uint8Array, offset: number, length: number): string {
  if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(length) || offset < 0 || length < 0 || offset + length * 2 > source.length) throw new RangeError("Invalid UTF-16LE slice");
  const chunks: string[] = [];
  const chunkSize = 4096;
  for (let start = 0; start < length; start += chunkSize) {
    const end = Math.min(length, start + chunkSize);
    const codes = new Array<number>(end - start);
    for (let index = start; index < end; index += 1) codes[index - start] = source[offset + index * 2]! | source[offset + index * 2 + 1]! << 8;
    chunks.push(String.fromCharCode(...codes));
  }
  return chunks.join("");
}

/** A reusable UTF-16 chunk builder for tokenization and piece-tree assembly. */
export class StringBuilder {
  private readonly buffer: Uint16Array;
  private readonly completedStrings: string[] = [];
  private bufferLength = 0;

  constructor(capacity: number) {
    if (!Number.isSafeInteger(capacity) || capacity < 1) throw new RangeError("StringBuilder capacity must be positive");
    this.buffer = new Uint16Array(capacity);
  }

  reset(): void {
    this.completedStrings.length = 0;
    this.bufferLength = 0;
  }

  build(): string {
    this.flushBuffer();
    return this.completedStrings.join("");
  }

  appendCharCode(charCode: number): void {
    if (!Number.isSafeInteger(charCode) || charCode < 0 || charCode > 0xffff) throw new RangeError("Character code must be a UTF-16 code unit");
    if (this.bufferLength === this.buffer.length) this.flushBuffer();
    this.buffer[this.bufferLength] = charCode;
    this.bufferLength += 1;
  }

  appendASCIICharCode(charCode: number): void {
    if (!Number.isSafeInteger(charCode) || charCode < 0 || charCode > 0x7f) throw new RangeError("Character code must be ASCII");
    this.appendCharCode(charCode);
  }

  appendString(value: string): void {
    if (value.length >= this.buffer.length) {
      this.flushBuffer();
      this.completedStrings.push(value);
      return;
    }
    if (this.bufferLength + value.length > this.buffer.length) this.flushBuffer();
    for (let index = 0; index < value.length; index += 1) this.buffer[this.bufferLength + index] = value.charCodeAt(index);
    this.bufferLength += value.length;
  }

  private flushBuffer(): void {
    if (this.bufferLength === 0) return;
    this.completedStrings.push(String.fromCharCode(...this.buffer.subarray(0, this.bufferLength)));
    this.bufferLength = 0;
  }
}

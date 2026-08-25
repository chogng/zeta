import type { JsonValue } from './jsonValue.js';

export enum JsonTokenKind {
	OpenBrace = 'openBrace',
	CloseBrace = 'closeBrace',
	OpenBracket = 'openBracket',
	CloseBracket = 'closeBracket',
	Comma = 'comma',
	Colon = 'colon',
	String = 'string',
	Number = 'number',
	True = 'true',
	False = 'false',
	Null = 'null',
	LineComment = 'lineComment',
	BlockComment = 'blockComment',
	Trivia = 'trivia',
	Unknown = 'unknown',
}

export interface JsonToken {
	readonly kind: JsonTokenKind;
	readonly offset: number;
	readonly length: number;
	readonly value?: string;
}

export interface JsonParseError {
	readonly message: string;
	readonly offset: number;
	readonly length: number;
}

export interface JsonParseOptions {
	readonly allowComments?: boolean;
	readonly allowTrailingComma?: boolean;
}

export interface JsonFormattingOptions {
	readonly tabSize: number;
	readonly insertSpaces: boolean;
	readonly eol?: '\n' | '\r\n';
}

interface JsonNodeBase {
	readonly offset: number;
	readonly length: number;
}

export interface JsonStringNode extends JsonNodeBase {
	readonly type: 'string';
	readonly value: string;
}

export interface JsonNumberNode extends JsonNodeBase {
	readonly type: 'number';
	readonly value: number;
}

export interface JsonBooleanNode extends JsonNodeBase {
	readonly type: 'boolean';
	readonly value: boolean;
}

export interface JsonNullNode extends JsonNodeBase {
	readonly type: 'null';
	readonly value: null;
}

export interface JsonPropertyNode extends JsonNodeBase {
	readonly type: 'property';
	readonly key: string;
	readonly keyNode: JsonStringNode;
	readonly valueNode: JsonValueNode | undefined;
}

export interface JsonObjectNode extends JsonNodeBase {
	readonly type: 'object';
	readonly properties: readonly JsonPropertyNode[];
}

export interface JsonArrayNode extends JsonNodeBase {
	readonly type: 'array';
	readonly items: readonly JsonValueNode[];
}

export type JsonValueNode = JsonStringNode | JsonNumberNode | JsonBooleanNode | JsonNullNode | JsonObjectNode | JsonArrayNode;
export type JsonNode = JsonValueNode | JsonPropertyNode;

export interface JsonDocument {
	readonly root: JsonValueNode | undefined;
	readonly value: JsonValue | undefined;
	readonly tokens: readonly JsonToken[];
	readonly errors: readonly JsonParseError[];
}

interface ScannedJson {
	readonly tokens: readonly JsonToken[];
	readonly errors: readonly JsonParseError[];
}

const numberPattern = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/u;

/** Scans JSON punctuation, values, comments, and trivia without discarding source ranges. */
export function scanJson(source: string, options: JsonParseOptions = {}): ScannedJson {
	if (typeof source !== 'string') throw new TypeError('JSON source must be text');
	const tokens: JsonToken[] = [];
	const errors: JsonParseError[] = [];
	let offset = 0;
	while (offset < source.length) {
		const start = offset;
		const character = source[offset]!;
		const punctuation = punctuationKind(character);
		if (punctuation) {
			offset += 1;
			tokens.push(token(punctuation, start, offset));
			continue;
		}
		if (isJsonWhitespace(character)) {
			do {
				offset += 1;
			} while (offset < source.length && isJsonWhitespace(source[offset]!));
			tokens.push(token(JsonTokenKind.Trivia, start, offset));
			continue;
		}
		if (character === '/' && (source[offset + 1] === '/' || source[offset + 1] === '*')) {
			const isLineComment = source[offset + 1] === '/';
			offset += 2;
			if (isLineComment) {
				while (offset < source.length && source[offset] !== '\r' && source[offset] !== '\n') offset += 1;
				tokens.push(token(JsonTokenKind.LineComment, start, offset));
			} else {
				while (offset < source.length && !(source[offset] === '*' && source[offset + 1] === '/')) offset += 1;
				if (offset >= source.length) {
					errors.push(parseError('Unterminated block comment', start, source.length - start));
				} else {
					offset += 2;
				}
				tokens.push(token(JsonTokenKind.BlockComment, start, offset));
			}
			if (!options.allowComments) errors.push(parseError('Comments are not permitted in JSON', start, offset - start));
			continue;
		}
		if (character === '"') {
			offset = scanString(source, start, errors);
			const raw = source.slice(start, offset);
			tokens.push({ kind: JsonTokenKind.String, offset: start, length: offset - start, value: decodeStringToken(raw) });
			continue;
		}
		const number = numberPattern.exec(source.slice(offset))?.[0];
		if (number) {
			offset += number.length;
			tokens.push({ kind: JsonTokenKind.Number, offset: start, length: number.length, value: number });
			continue;
		}
		const keyword = keywordKind(source, offset);
		if (keyword) {
			offset += keyword.length;
			tokens.push(token(keyword.kind, start, offset));
			continue;
		}
		offset += 1;
		while (offset < source.length && !isTokenBoundary(source[offset]!)) offset += 1;
		tokens.push(token(JsonTokenKind.Unknown, start, offset));
		errors.push(parseError(`Unexpected token '${source.slice(start, offset)}'`, start, offset - start));
	}
	return Object.freeze({ tokens: Object.freeze(tokens), errors: Object.freeze(errors) });
}

/** Parses strict JSON or JSONC while retaining a source-ranged syntax tree. */
export function parseJsonDocument(source: string, options: JsonParseOptions = {}): JsonDocument {
	const scanned = scanJson(source, options);
	const parser = new JsonParser(source, scanned.tokens, scanned.errors, options);
	return parser.parse();
}

/** Formats valid JSON or JSONC from tokens while preserving comment text and trailing commas. */
export function formatJson(source: string, options: JsonFormattingOptions): string {
	if (!Number.isSafeInteger(options.tabSize) || options.tabSize < 1) throw new RangeError('JSON formatting tab size must be positive');
	const document = parseJsonDocument(source, { allowComments: true, allowTrailingComma: true });
	if (document.errors.length > 0) throw new TypeError(`Cannot format invalid JSONC: ${document.errors[0]!.message}`);
	const tokens = scanJson(source, { allowComments: true, allowTrailingComma: true }).tokens.filter(candidate => candidate.kind !== JsonTokenKind.Trivia);
	const eol = options.eol ?? (source.includes('\r\n') ? '\r\n' : '\n');
	const indentation = options.insertSpaces ? ' '.repeat(options.tabSize) : '\t';
	let result = '';
	let level = 0;
	let isLineStart = true;
	const write = (value: string): void => {
		if (isLineStart) {
			result += indentation.repeat(level);
			isLineStart = false;
		}
		result += value;
	};
	const newline = (): void => {
		result = result.replace(/[\t ]+$/u, '');
		if (!result.endsWith(eol)) result += eol;
		isLineStart = true;
	};
	for (let index = 0; index < tokens.length; index += 1) {
		const current = tokens[index]!;
		const next = tokens[index + 1];
		const previous = tokens[index - 1];
		const raw = source.slice(current.offset, current.offset + current.length);
		switch (current.kind) {
			case JsonTokenKind.OpenBrace:
			case JsonTokenKind.OpenBracket:
				write(raw);
				if (!next || !isMatchingClose(current.kind, next.kind)) {
					level += 1;
					newline();
				}
				break;
			case JsonTokenKind.CloseBrace:
			case JsonTokenKind.CloseBracket:
				if (!previous || !isMatchingClose(previous.kind, current.kind)) {
					level = Math.max(0, level - 1);
					if (!isLineStart) newline();
				}
				write(raw);
				break;
			case JsonTokenKind.Comma:
				write(raw);
				newline();
				break;
			case JsonTokenKind.Colon:
				write(': ');
				break;
			case JsonTokenKind.LineComment:
				if (!isLineStart && !/[\t ]$/u.test(result)) result += ' ';
				write(raw);
				newline();
				break;
			case JsonTokenKind.BlockComment:
				if (!isLineStart && !/[\t ]$/u.test(result)) result += ' ';
				write(raw.replace(/\r?\n/gu, eol));
				if (next && next.kind !== JsonTokenKind.Comma) newline();
				else result += ' ';
				break;
			default:
				write(raw);
				break;
		}
	}
	return `${result.trimEnd()}${eol}`;
}

/** Returns the deepest syntax node containing one source offset. */
export function findJsonNodeAtOffset(root: JsonValueNode | undefined, offset: number): JsonNode | undefined {
	if (!root || offset < root.offset || offset > root.offset + root.length) return undefined;
	if (root.type === 'object') {
		for (const property of root.properties) {
			if (offset < property.offset || offset > property.offset + property.length) continue;
			if (offset <= property.keyNode.offset + property.keyNode.length) return property.keyNode;
			return findJsonNodeAtOffset(property.valueNode, offset) ?? property;
		}
	}
	if (root.type === 'array') {
		for (const item of root.items) {
			const nested = findJsonNodeAtOffset(item, offset);
			if (nested) return nested;
		}
	}
	return root;
}

/** Returns the property/index path of one value node. */
export function getJsonNodePath(root: JsonValueNode | undefined, target: JsonValueNode): readonly (string | number)[] | undefined {
	if (!root) return undefined;
	if (root === target) return Object.freeze([]);
	if (root.type === 'object') {
		for (const property of root.properties) {
			if (!property.valueNode) continue;
			const nested = getJsonNodePath(property.valueNode, target);
			if (nested) return Object.freeze([property.key, ...nested]);
		}
	}
	if (root.type === 'array') {
		for (let index = 0; index < root.items.length; index += 1) {
			const nested = getJsonNodePath(root.items[index], target);
			if (nested) return Object.freeze([index, ...nested]);
		}
	}
	return undefined;
}

/** Updates one top-level object property while preserving unrelated JSONC source. */
export function editJsonObjectProperty(source: string, key: string, value: unknown | undefined): string {
	if (typeof key !== 'string' || key.length === 0) throw new TypeError('JSON property key must not be empty');
	const document = parseJsonDocument(source, { allowComments: true, allowTrailingComma: true });
	if (document.errors.length > 0) throw new TypeError(`Cannot edit invalid JSONC: ${document.errors[0]!.message}`);
	if (document.root?.type !== 'object') throw new TypeError('JSONC root must be an object');
	const property = document.root.properties.find(candidate => candidate.key === key);
	if (value === undefined) return property ? removeObjectProperty(source, document, property) : source;
	const formatting = detectFormatting(source, document.root);
	const serialized = formatJsonValue(value, formatting.indent, formatting.eol);
	if (property?.valueNode) {
		if (JSON.stringify(propertyValue(property.valueNode)) === JSON.stringify(value)) return source;
		const propertyIndent = indentationAt(source, property.offset);
		const replacement = indentMultiline(serialized, propertyIndent, formatting.eol);
		return replaceSource(source, property.valueNode.offset, property.valueNode.length, replacement);
	}
	if (property) throw new TypeError(`JSON property '${key}' has no value`);
	return insertObjectProperty(source, document, key, serialized, formatting);
}

function punctuationKind(character: string): JsonTokenKind | undefined {
	switch (character) {
		case '{': return JsonTokenKind.OpenBrace;
		case '}': return JsonTokenKind.CloseBrace;
		case '[': return JsonTokenKind.OpenBracket;
		case ']': return JsonTokenKind.CloseBracket;
		case ',': return JsonTokenKind.Comma;
		case ':': return JsonTokenKind.Colon;
		default: return undefined;
	}
}

function token(kind: JsonTokenKind, offset: number, end: number): JsonToken {
	return Object.freeze({ kind, offset, length: end - offset });
}

function parseError(message: string, offset: number, length: number): JsonParseError {
	return Object.freeze({ message, offset, length: Math.max(length, 1) });
}

function scanString(source: string, start: number, errors: JsonParseError[]): number {
	let offset = start + 1;
	while (offset < source.length) {
		const character = source[offset]!;
		if (character === '"') return offset + 1;
		if (character === '\r' || character === '\n') {
			errors.push(parseError('Unterminated string', start, offset - start));
			return offset;
		}
		if (character.charCodeAt(0) <= 0x1f) errors.push(parseError('Unescaped control character in string', offset, 1));
		if (character === '\\') {
			offset += 1;
			if (offset >= source.length) break;
			const escaped = source[offset]!;
			if (escaped === 'u') {
				const digits = source.slice(offset + 1, offset + 5);
				if (!/^[0-9A-Fa-f]{4}$/u.test(digits)) errors.push(parseError('Invalid Unicode escape sequence', offset - 1, Math.min(6, source.length - offset + 1)));
				offset += Math.min(5, source.length - offset);
				continue;
			}
			if (!'"\\/bfnrt'.includes(escaped)) errors.push(parseError(`Invalid escape sequence '\\${escaped}'`, offset - 1, 2));
		}
		offset += 1;
	}
	errors.push(parseError('Unterminated string', start, source.length - start));
	return source.length;
}

function decodeStringToken(raw: string): string {
	try {
		return JSON.parse(raw) as string;
	} catch {
		const content = raw.endsWith('"') ? raw.slice(1, -1) : raw.slice(1);
		try {
			return JSON.parse(`"${content.replace(/"/gu, '\\"')}"`) as string;
		} catch {
			return content;
		}
	}
}

function keywordKind(source: string, offset: number): { readonly kind: JsonTokenKind; readonly length: number } | undefined {
	for (const [word, kind] of [['true', JsonTokenKind.True], ['false', JsonTokenKind.False], ['null', JsonTokenKind.Null]] as const) {
		if (source.startsWith(word, offset) && isTokenBoundary(source[offset + word.length] ?? '')) return { kind, length: word.length };
	}
	return undefined;
}

function isTokenBoundary(character: string): boolean {
	return character === '' || isJsonWhitespace(character) || /[{}\[\],:]/u.test(character);
}

function isJsonWhitespace(character: string): boolean {
	return character === ' ' || character === '\t' || character === '\r' || character === '\n';
}

class JsonParser {
	private readonly significantTokens: readonly JsonToken[];
	private readonly errors: JsonParseError[];
	private index = 0;

	constructor(
		private readonly source: string,
		tokens: readonly JsonToken[],
		scanErrors: readonly JsonParseError[],
		private readonly options: JsonParseOptions,
	) {
		this.significantTokens = tokens.filter(candidate => !isTrivia(candidate.kind));
		this.errors = [...scanErrors];
	}

	public parse(): JsonDocument {
		const root = this.parseValue();
		if (!root && this.significantTokens.length === 0) this.errors.push(parseError('Expected a JSON value', 0, 1));
		if (root && this.current()) this.errors.push(parseError('Unexpected content after the root value', this.current()!.offset, this.current()!.length));
		return Object.freeze({
			root,
			value: root ? propertyValue(root) : undefined,
			tokens: Object.freeze([...this.significantTokens]),
			errors: Object.freeze(this.deduplicatedErrors()),
		});
	}

	private parseValue(): JsonValueNode | undefined {
		const current = this.current();
		if (!current) return undefined;
		switch (current.kind) {
			case JsonTokenKind.OpenBrace: return this.parseObject();
			case JsonTokenKind.OpenBracket: return this.parseArray();
			case JsonTokenKind.String:
				this.index += 1;
				return Object.freeze({ type: 'string', value: current.value ?? '', offset: current.offset, length: current.length });
			case JsonTokenKind.Number:
				this.index += 1;
				return Object.freeze({ type: 'number', value: Number(current.value), offset: current.offset, length: current.length });
			case JsonTokenKind.True:
			case JsonTokenKind.False:
				this.index += 1;
				return Object.freeze({ type: 'boolean', value: current.kind === JsonTokenKind.True, offset: current.offset, length: current.length });
			case JsonTokenKind.Null:
				this.index += 1;
				return Object.freeze({ type: 'null', value: null, offset: current.offset, length: current.length });
			default:
				this.errors.push(parseError('Expected a JSON value', current.offset, current.length));
				this.index += 1;
				return undefined;
		}
	}

	private parseObject(): JsonObjectNode {
		const open = this.consume(JsonTokenKind.OpenBrace)!;
		const properties: JsonPropertyNode[] = [];
		while (this.current() && this.current()!.kind !== JsonTokenKind.CloseBrace) {
			const key = this.current()!;
			if (key.kind !== JsonTokenKind.String) {
				this.errors.push(parseError('Expected an object property name', key.offset, key.length));
				this.recover(JsonTokenKind.Comma, JsonTokenKind.CloseBrace);
				if (this.consume(JsonTokenKind.Comma)) continue;
				break;
			}
			this.index += 1;
			const keyNode = Object.freeze<JsonStringNode>({ type: 'string', value: key.value ?? '', offset: key.offset, length: key.length });
			if (!this.consume(JsonTokenKind.Colon)) {
				const current = this.current();
				this.errors.push(parseError("Expected ':' after the property name", current?.offset ?? key.offset + key.length, current?.length ?? 1));
			}
			const valueNode = this.parseValue();
			const end = valueNode ? valueNode.offset + valueNode.length : key.offset + key.length;
			properties.push(Object.freeze({ type: 'property', key: keyNode.value, keyNode, valueNode, offset: key.offset, length: end - key.offset }));
			const comma = this.consume(JsonTokenKind.Comma);
			if (comma) {
				if (this.current()?.kind === JsonTokenKind.CloseBrace && !this.options.allowTrailingComma) {
					this.errors.push(parseError('Trailing commas are not permitted in JSON', comma.offset, comma.length));
				}
				continue;
			}
			if (this.current()?.kind !== JsonTokenKind.CloseBrace) {
				const current = this.current();
				this.errors.push(parseError("Expected ',' or '}' after the property value", current?.offset ?? end, current?.length ?? 1));
				this.recover(JsonTokenKind.Comma, JsonTokenKind.CloseBrace);
				this.consume(JsonTokenKind.Comma);
			}
		}
		const close = this.consume(JsonTokenKind.CloseBrace);
		if (!close) this.errors.push(parseError("Expected '}'", this.source.length, 1));
		const end = close ? close.offset + close.length : this.source.length;
		return Object.freeze({ type: 'object', properties: Object.freeze(properties), offset: open.offset, length: end - open.offset });
	}

	private parseArray(): JsonArrayNode {
		const open = this.consume(JsonTokenKind.OpenBracket)!;
		const items: JsonValueNode[] = [];
		while (this.current() && this.current()!.kind !== JsonTokenKind.CloseBracket) {
			const before = this.index;
			const value = this.parseValue();
			if (value) items.push(value);
			if (this.index === before) this.index += 1;
			const comma = this.consume(JsonTokenKind.Comma);
			if (comma) {
				if (this.current()?.kind === JsonTokenKind.CloseBracket && !this.options.allowTrailingComma) {
					this.errors.push(parseError('Trailing commas are not permitted in JSON', comma.offset, comma.length));
				}
				continue;
			}
			if (this.current()?.kind !== JsonTokenKind.CloseBracket) {
				const current = this.current();
				this.errors.push(parseError("Expected ',' or ']' after the array item", current?.offset ?? this.source.length, current?.length ?? 1));
				this.recover(JsonTokenKind.Comma, JsonTokenKind.CloseBracket);
				this.consume(JsonTokenKind.Comma);
			}
		}
		const close = this.consume(JsonTokenKind.CloseBracket);
		if (!close) this.errors.push(parseError("Expected ']'", this.source.length, 1));
		const end = close ? close.offset + close.length : this.source.length;
		return Object.freeze({ type: 'array', items: Object.freeze(items), offset: open.offset, length: end - open.offset });
	}

	private current(): JsonToken | undefined {
		return this.significantTokens[this.index];
	}

	private consume(kind: JsonTokenKind): JsonToken | undefined {
		const current = this.current();
		if (current?.kind !== kind) return undefined;
		this.index += 1;
		return current;
	}

	private recover(...kinds: readonly JsonTokenKind[]): void {
		while (this.current() && !kinds.includes(this.current()!.kind)) this.index += 1;
	}

	private deduplicatedErrors(): readonly JsonParseError[] {
		const seen = new Set<string>();
		return this.errors.filter(error => {
			const key = `${error.offset}:${error.length}:${error.message}`;
			if (seen.has(key)) return false;
			seen.add(key);
			return true;
		});
	}
}

function isTrivia(kind: JsonTokenKind): boolean {
	return kind === JsonTokenKind.Trivia || kind === JsonTokenKind.LineComment || kind === JsonTokenKind.BlockComment;
}

function isMatchingClose(open: JsonTokenKind, close: JsonTokenKind): boolean {
	return open === JsonTokenKind.OpenBrace && close === JsonTokenKind.CloseBrace || open === JsonTokenKind.OpenBracket && close === JsonTokenKind.CloseBracket;
}

function propertyValue(node: JsonValueNode): JsonValue {
	switch (node.type) {
		case 'string': return node.value;
		case 'number': return node.value;
		case 'boolean': return node.value;
		case 'null': return null;
		case 'array': return node.items.map(propertyValue);
		case 'object': {
			const result: Record<string, JsonValue> = {};
			for (const property of node.properties) {
				if (property.valueNode) Object.defineProperty(result, property.key, { value: propertyValue(property.valueNode), enumerable: true, configurable: true, writable: true });
			}
			return result;
		}
	}
}

interface JsonFormatting {
	readonly eol: string;
	readonly indent: string;
	readonly rootIndent: string;
}

function detectFormatting(source: string, root: JsonObjectNode): JsonFormatting {
	const eol = source.includes('\r\n') ? '\r\n' : '\n';
	const rootIndent = indentationAt(source, root.offset);
	const firstProperty = root.properties[0];
	const propertyIndent = firstProperty ? indentationAt(source, firstProperty.offset) : '';
	const indent = propertyIndent.startsWith(rootIndent) && propertyIndent.length > rootIndent.length
		? propertyIndent.slice(rootIndent.length)
		: '\t';
	return { eol, indent, rootIndent };
}

function formatJsonValue(value: unknown, indent: string, eol: string): string {
	const serialized = JSON.stringify(value, null, indent);
	if (serialized === undefined) throw new TypeError('JSON property value must be serializable');
	return serialized.replace(/\n/gu, eol);
}

function indentationAt(source: string, offset: number): string {
	const lineStart = Math.max(source.lastIndexOf('\n', offset - 1) + 1, 0);
	return /^[\t ]*/u.exec(source.slice(lineStart, offset))?.[0] ?? '';
}

function indentMultiline(value: string, propertyIndent: string, eol: string): string {
	return value.split(eol).map((line, index) => index === 0 ? line : `${propertyIndent}${line}`).join(eol);
}

function insertObjectProperty(source: string, document: JsonDocument, key: string, serialized: string, formatting: JsonFormatting): string {
	const root = document.root as JsonObjectNode;
	const closeOffset = root.offset + root.length - 1;
	const propertyIndent = `${formatting.rootIndent}${formatting.indent}`;
	const value = indentMultiline(serialized, propertyIndent, formatting.eol);
	const propertySource = `${JSON.stringify(key)}: ${value}`;
	const lastProperty = root.properties.at(-1);
	if (!lastProperty) {
		const inner = source.slice(root.offset + 1, closeOffset);
		if (/^\s*$/u.test(inner)) {
			return replaceSource(source, root.offset + 1, inner.length, `${formatting.eol}${propertyIndent}${propertySource}${formatting.eol}${formatting.rootIndent}`);
		}
		const separator = inner.endsWith('\n') || inner.endsWith('\r') ? '' : formatting.eol;
		return replaceSource(source, closeOffset, 0, `${separator}${propertyIndent}${propertySource}${formatting.eol}${formatting.rootIndent}`);
	}
	const trailingComma = document.tokens.find(candidate => candidate.kind === JsonTokenKind.Comma && candidate.offset >= lastProperty.offset + lastProperty.length && candidate.offset < closeOffset);
	if (trailingComma) {
		return replaceSource(source, trailingComma.offset + trailingComma.length, 0, `${formatting.eol}${propertyIndent}${propertySource},`);
	}
	return replaceSource(source, lastProperty.offset + lastProperty.length, 0, `,${formatting.eol}${propertyIndent}${propertySource}`);
}

function removeObjectProperty(source: string, document: JsonDocument, property: JsonPropertyNode): string {
	const root = document.root as JsonObjectNode;
	const index = root.properties.indexOf(property);
	const next = root.properties[index + 1];
	const closeOffset = root.offset + root.length - 1;
	const followingLimit = next?.offset ?? closeOffset;
	const followingComma = document.tokens.find(candidate => candidate.kind === JsonTokenKind.Comma && candidate.offset >= property.offset + property.length && candidate.offset < followingLimit);
	if (followingComma) {
		const withoutComma = replaceSource(source, followingComma.offset, followingComma.length, '');
		return replaceSource(withoutComma, property.offset, property.length, '');
	}
	const previous = root.properties[index - 1];
	if (!previous) return replaceSource(source, property.offset, property.length, '');
	const separator = [...document.tokens].reverse().find(candidate => candidate.kind === JsonTokenKind.Comma && candidate.offset >= previous.offset + previous.length && candidate.offset < property.offset);
	const withoutProperty = replaceSource(source, property.offset, property.length, '');
	return separator ? replaceSource(withoutProperty, separator.offset, separator.length, '') : withoutProperty;
}

function replaceSource(source: string, offset: number, length: number, replacement: string): string {
	return `${source.slice(0, offset)}${replacement}${source.slice(offset + length)}`;
}

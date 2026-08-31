import { addDisposableListener } from '../../../../base/browser/dom.js';
import { isFirefox } from '../../../../base/browser/browser.js';
import { UriList } from '../../../../base/common/dataTransfer.js';
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { isWindows } from '../../../../base/common/platform.js';
import { generateUuid } from '../../../../base/common/uuid.js';
import { type IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { DeleteOperations } from '../../../common/cursor/cursorDeleteOperations.js';
import { TypeOperations } from "../../../common/cursor/cursorTypeOperations.js";
import { type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type Selection } from "../../../common/core/selection.js";
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { type TextModel } from "../../../common/model/textModel.js";
import { type View } from "../../../browser/view.js";
import { AbstractEditContext } from "../../../browser/controller/editContext/editContext.js";
import { createEditorClipboardCopyEvent, createClipboardPasteEvent, InMemoryClipboardMetadataManager, readEditorClipboardText, type ClipboardStoredMetadata, type IEditorClipboardCopyEvent, type IClipboardPasteEvent, type IReadableClipboardData, type IWritableClipboardData } from '../../../browser/controller/editContext/clipboardUtils.js';
import { SemanticTokenPresentation, type SemanticTokenSource } from "../../../browser/viewParts/viewLines/viewLine.js";
import { TEXT_FILE_TRANSFER_MAX_BYTES, selectTextFileTransfer } from '../../dropOrPasteInto/browser/textFileTransfer.js';

export const EDITOR_CLIPBOARD_MIME = 'application/x-stanza-editor';
export const EDITOR_HTML_CLIPBOARD_MIME = 'text/html';

export enum ClipboardLineEnding {
	LF = '\n',
	CRLF = '\r\n',
}

export enum EditorEmptySelectionClipboardPolicy {
	Ignore = 'ignore',
	Line = 'line',
}

export enum EditorClipboardPasteMode {
	Selection = 'selection',
	Line = 'line',
}

interface EditorClipboardEntry {
	readonly text: string;
	readonly sourceRange: Range;
	readonly pasteMode: EditorClipboardPasteMode;
}

interface EditorClipboardPayload {
	readonly plainText: string;
	readonly html: string;
	readonly metadata: string;
	readonly editorMetadata: ClipboardStoredMetadata;
}

interface EditorClipboardPasteData {
	readonly texts: readonly string[];
	readonly modes: readonly EditorClipboardPasteMode[];
}

interface ClipboardMetadata {
	readonly version: 2;
	readonly selectionTexts: readonly string[];
	readonly pasteModes: readonly EditorClipboardPasteMode[];
}

export interface ClipboardControllerOptions {
	readonly lineEnding?: ClipboardLineEnding;
	readonly emptySelectionPolicy?: EditorEmptySelectionClipboardPolicy;
	/** Optional current token source used only for portable HTML copy output. */
	readonly semanticTokens?: SemanticTokenSource;
	/** Rejects cut and paste while another input adapter owns a protected edit. */
	readonly isEditingAllowed?: () => boolean;
}

/**
 * Routes native clipboard events through Stanza's selection-aware commands.
 */
export class ClipboardController extends Disposable {
	private readonly lineEnding: ClipboardLineEnding;
	private readonly emptySelectionPolicy: EditorEmptySelectionClipboardPolicy;
	private readonly semanticTokens: SemanticTokenSource | undefined;
	private readonly isEditingAllowed: () => boolean;
	private asynchronousPasteRequest = 0;

	constructor(
		target: AbstractEditContext | HTMLElement,
		private readonly viewport: View,
		private readonly selectionController: CursorsController,
		private readonly clipboardService: IClipboardService,
		options: ClipboardControllerOptions = {},
	) {
		super();
		const editContext = target instanceof AbstractEditContext ? target : undefined;
		if (target instanceof AbstractEditContext) this.element = target.domNode;
		else this.element = target;
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Stanza clipboard and selection controllers must share one text model",
			);
		}
		if (options.semanticTokens && options.semanticTokens.textModel !== viewport.textModel) {
			this.dispose();
			throw new TypeError("Stanza clipboard semantic tokens must share the viewport text model");
		}
		if (options.isEditingAllowed !== undefined && typeof options.isEditingAllowed !== "function") {
			this.dispose();
			throw new TypeError("Stanza clipboard edit gate must be a function");
		}
		if (!clipboardService || typeof clipboardService.readText !== 'function' || typeof clipboardService.writeText !== 'function') {
			this.dispose();
			throw new TypeError('Editor clipboard requires the platform clipboard service');
		}
		this.lineEnding = resolveClipboardLineEnding(options.lineEnding);
		this.emptySelectionPolicy = readEmptySelectionPolicy(
			options.emptySelectionPolicy,
		);
		this.semanticTokens = options.semanticTokens;
		this.isEditingAllowed = options.isEditingAllowed ?? (() => true);
		this._register(toDisposable(() => {
			this.asynchronousPasteRequest += 1;
		}));
		if (editContext) {
			this._register(editContext.onWillCopy(event => this.handleCopy(event)));
			this._register(editContext.onWillCut(event => this.handleCut(event)));
			this._register(editContext.onWillPaste(event => this.handlePaste(event)));
		} else {
			this._register(addDisposableListener<ClipboardEvent>(
				this.element,
				"copy",
				event => this.handleCopy(createEditorClipboardCopyEvent(event, false)),
			));
			this._register(addDisposableListener<ClipboardEvent>(
				this.element,
				"cut",
				event => this.handleCut(createEditorClipboardCopyEvent(event, true)),
			));
			this._register(addDisposableListener<ClipboardEvent>(
				this.element,
				"paste",
				event => this.handlePaste(createClipboardPasteEvent(event)),
			));
		}
	}

	private readonly element: HTMLElement;

	private handleCopy(event: IEditorClipboardCopyEvent): void {
		if (event.isHandled || event.browserEvent.defaultPrevented) return;
		const entries = getEditorClipboardEntries(
			this.viewport.textModel,
			this.selectionController.selections,
			this.emptySelectionPolicy,
		);
		if (event.hasClipboardData && this.writeClipboard(event.clipboardData, entries)) {
			event.setHandled();
			return;
		}
		this.writeSystemClipboard(event, entries);
	}

	private handleCut(event: IEditorClipboardCopyEvent): void {
		if (event.isHandled || event.browserEvent.defaultPrevented) return;
		if (!this.isEditingAllowed()) {
			event.setHandled();
			return;
		}
		const entries = getEditorClipboardEntries(
			this.viewport.textModel,
			this.selectionController.selections,
			this.emptySelectionPolicy,
		);
		if (event.hasClipboardData && this.writeClipboard(event.clipboardData, entries)) {
			event.setHandled();
			this.executeCut();
			return;
		}
		this.writeSystemClipboard(event, entries, true);
	}

	private handlePaste(event: IClipboardPasteEvent): void {
		const nativeClipboard = event.clipboardData;
		if (event.isHandled || event.browserEvent?.defaultPrevented) return;
		if (!this.isEditingAllowed()) {
			event.setHandled();
			return;
		}
		const text = readEditorClipboardText(nativeClipboard, this.element.ownerDocument);
		const clipboardData = readEditorClipboardPasteData(
			nativeClipboard,
			this.selectionController.selections.length,
		);
		if (text.length === 0 && !clipboardData?.texts.some(value => value.length > 0)) {
			const uriList = readUriList(nativeClipboard.getData('text/uri-list'));
			if (uriList) {
				event.setHandled();
				this.selectionController.execute(TypeOperations.paste(this.viewport.textModel, this.selectionController.selections, uriList));
				this.afterEdit();
				return;
			}
			if (this.pasteTextFile(event)) return;
			this.pasteSystemText(event);
			return;
		}
		const command = clipboardData
			? createMetadataPasteCommand(
				this.viewport.textModel,
				this.selectionController.selections,
				clipboardData,
			)
			: TypeOperations.paste(
				this.viewport.textModel,
				this.selectionController.selections,
				text,
			);
		event.setHandled();
		this.selectionController.execute(command);
		this.afterEdit();
	}

	private pasteSystemText(event: IClipboardPasteEvent): void {
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		event.setHandled();
		void this.clipboardService.readText().then(text => {
			if (
				text.length === 0 ||
				this.isDisposed ||
				request !== this.asynchronousPasteRequest ||
				!this.isEditingAllowed() ||
				model.version !== expectedVersion ||
				!selectionSetsEqual(this.selectionController.selections, expectedSelections)
			) {
				return;
			}
			this.selectionController.execute(TypeOperations.paste(model, expectedSelections, text));
			this.afterEdit();
		}).catch(() => {
			// Clipboard permission failures leave the model unchanged.
		});
	}

	private writeSystemClipboard(event: IEditorClipboardCopyEvent, entries: readonly EditorClipboardEntry[], cut = false): boolean {
		if (!entries.some(entry => entry.text.length > 0)) return false;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		const payload = this.createClipboardPayload(entries);
		storeEditorClipboardMetadata(payload.plainText, payload.editorMetadata);
		event.setHandled();
		void this.clipboardService.writeText(payload.plainText).then(() => {
			if (!cut || this.isDisposed || request !== this.asynchronousPasteRequest || !this.isEditingAllowed() || model.version !== expectedVersion || !selectionSetsEqual(this.selectionController.selections, expectedSelections)) return;
			this.executeCut();
		}).catch(() => {
			// Permission failures must never mutate the model, especially for cut.
		});
		return true;
	}

	private executeCut(): void {
		this.selectionController.execute(DeleteOperations.cut(
			this.viewport.textModel,
			this.selectionController.selections,
			this.selectionController.selections.map(selection => createClipboardEntry(
				this.viewport.textModel,
				selection,
				this.emptySelectionPolicy,
			).sourceRange),
		));
		this.afterEdit();
	}

	private writeClipboard(clipboardData: IWritableClipboardData, entries: readonly EditorClipboardEntry[]): boolean {
		if (!entries.some(entry => entry.text.length > 0)) return false;
		const payload = this.createClipboardPayload(entries);
		try {
			clipboardData.setData(
				"text/plain",
				payload.plainText,
			);
		} catch {
			return false;
		}
		try {
			clipboardData.setData(
				EDITOR_CLIPBOARD_MIME,
				payload.metadata,
			);
		} catch {
			// Plain text remains portable when a browser rejects custom MIME data.
		}
		try {
			clipboardData.setData(EDITOR_HTML_CLIPBOARD_MIME, payload.html);
		} catch {
			// Plain text remains authoritative when a browser rejects HTML clipboard data.
		}
		try {
			clipboardData.setData('vscode-editor-data', JSON.stringify(payload.editorMetadata));
		} catch {
			// Plain text remains authoritative when custom metadata is rejected.
		}
		storeEditorClipboardMetadata(payload.plainText, payload.editorMetadata);
		return true;
	}

	private createClipboardPayload(entries: readonly EditorClipboardEntry[]): EditorClipboardPayload {
		return createEditorClipboardPayload(entries, this.lineEnding, this.semanticTokens, this.element.ownerDocument);
	}

	private pasteTextFile(event: IClipboardPasteEvent): boolean {
		const file = selectTextFileTransfer(event.clipboardData.files);
		if (!file) return false;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		event.setHandled();
		void file.text().then(text => {
			if (text.length > TEXT_FILE_TRANSFER_MAX_BYTES || this.isDisposed || request !== this.asynchronousPasteRequest || !this.isEditingAllowed() || model.version !== expectedVersion || !selectionSetsEqual(this.selectionController.selections, expectedSelections)) return;
			this.selectionController.execute(TypeOperations.paste(model, expectedSelections, text));
			this.afterEdit();
		}).catch(() => {
			// The supplied file could not be decoded as text.
		});
		return true;
	}

	private afterEdit(): void {
		if ("value" in this.element && typeof (this.element as { readonly value?: unknown }).value === "string") {
			(this.element as HTMLTextAreaElement).value = "";
		}
		this.viewport.revealPosition(
			this.selectionController.selections[0]!.getPosition(),
		);
	}
}

function getEditorClipboardEntries(model: TextModel, selections: readonly Selection[], policy: EditorEmptySelectionClipboardPolicy): readonly EditorClipboardEntry[] {
	if (!Object.values(EditorEmptySelectionClipboardPolicy).includes(policy)) {
		throw new TypeError('Unknown editor empty-selection clipboard policy');
	}
	return Object.freeze(
		selections
			.map(selection => createClipboardEntry(model, selection, policy))
			.sort((left, right) => Range.compareRangesUsingStarts(left.sourceRange, right.sourceRange)),
	);
}

function createClipboardEntry(model: TextModel, selection: Selection, policy: EditorEmptySelectionClipboardPolicy): EditorClipboardEntry {
	if (!selection.isEmpty()) {
		return Object.freeze({
			text: model.getTextInRange(selection),
			sourceRange: selection,
			pasteMode: EditorClipboardPasteMode.Selection,
		});
	}
	if (policy === EditorEmptySelectionClipboardPolicy.Ignore) {
		return Object.freeze({
			text: '',
			sourceRange: selection,
			pasteMode: EditorClipboardPasteMode.Selection,
		});
	}
	const lineNumber = selection.positionLineNumber;
	if (lineNumber < model.lineCount) {
		const sourceRange = Range.fromPositions(new Position(lineNumber, 1), new Position(lineNumber + 1, 1));
		return Object.freeze({ text: model.getTextInRange(sourceRange), sourceRange, pasteMode: EditorClipboardPasteMode.Line });
	}
	const lineText = model.getLineContent(lineNumber);
	const sourceRange = lineNumber === 1
		? Range.fromPositions(new Position(1, 1), new Position(1, lineText.length + 1))
		: Range.fromPositions(new Position(lineNumber - 1, model.getLineContent(lineNumber - 1).length + 1), new Position(lineNumber, lineText.length + 1));
	return Object.freeze({ text: `${lineText}\n`, sourceRange, pasteMode: EditorClipboardPasteMode.Line });
}

function resolveClipboardLineEnding(lineEnding: ClipboardLineEnding | undefined): ClipboardLineEnding {
	const resolved = lineEnding ?? (isWindows ? ClipboardLineEnding.CRLF : ClipboardLineEnding.LF);
	if (!Object.values(ClipboardLineEnding).includes(resolved)) throw new TypeError('Unknown editor clipboard line ending');
	return resolved;
}

function createEditorClipboardPayload(entries: readonly EditorClipboardEntry[], lineEnding: ClipboardLineEnding, tokens: SemanticTokenSource | undefined, ownerDocument: Document): EditorClipboardPayload {
	const metadata: ClipboardMetadata = {
		version: 2,
		selectionTexts: entries.map(entry => entry.text),
		pasteModes: entries.map(entry => entry.pasteMode),
	};
	return Object.freeze({
		plainText: joinClipboardEntries(entries, lineEnding),
		html: createSyntaxClipboardHtml(entries, lineEnding, tokens, ownerDocument),
		metadata: JSON.stringify(metadata),
		editorMetadata: Object.freeze({
			version: 1,
			id: generateUuid(),
			isFromEmptySelection: entries.length === 1 && entries[0]!.pasteMode === EditorClipboardPasteMode.Line,
			multicursorText: entries.length > 1 ? entries.map(entry => entry.text) : null,
			mode: tokens?.textModel.getLanguageId() ?? null,
		}),
	});
}

function storeEditorClipboardMetadata(text: string, metadata: ClipboardStoredMetadata): void {
	InMemoryClipboardMetadataManager.INSTANCE.set(isFirefox ? text.replace(/\r\n/g, '\n') : text, metadata);
}

function readEditorClipboardPasteData(clipboardData: IReadableClipboardData, selectionCount: number): EditorClipboardPasteData | undefined {
	let parsed: unknown;
	try {
		const raw = clipboardData.getData(EDITOR_CLIPBOARD_MIME);
		if (!raw) return undefined;
		parsed = JSON.parse(raw);
	} catch {
		return undefined;
	}
	if (typeof parsed !== 'object' || parsed === null || !('version' in parsed) || (parsed.version !== 1 && parsed.version !== 2) || !('selectionTexts' in parsed) || !Array.isArray(parsed.selectionTexts) || parsed.selectionTexts.length !== selectionCount || parsed.selectionTexts.some(text => typeof text !== 'string')) {
		return undefined;
	}
	const texts = parsed.selectionTexts as string[];
	let modes = parsed.version === 2 && 'pasteModes' in parsed && Array.isArray(parsed.pasteModes) && parsed.pasteModes.length === selectionCount && parsed.pasteModes.every(mode => Object.values(EditorClipboardPasteMode).includes(mode))
		? parsed.pasteModes as EditorClipboardPasteMode[]
		: texts.map(() => EditorClipboardPasteMode.Selection);
	if (modes.some((mode, index) => mode === EditorClipboardPasteMode.Line && !texts[index]!.endsWith('\n'))) {
		modes = texts.map(() => EditorClipboardPasteMode.Selection);
	}
	return Object.freeze({ texts: Object.freeze([...texts]), modes: Object.freeze([...modes]) });
}

function joinClipboardEntries(entries: readonly EditorClipboardEntry[], lineEnding: ClipboardLineEnding): string {
	const included = entries.filter(entry => entry.text.length > 0);
	let result = '';
	let previousMode: EditorClipboardPasteMode | undefined;
	for (const entry of included) {
		if (result.length > 0 && previousMode !== EditorClipboardPasteMode.Line) result += lineEnding;
		result += toExternalLineEndings(entry.text, lineEnding);
		previousMode = entry.pasteMode;
	}
	return result;
}

function createSyntaxClipboardHtml(entries: readonly EditorClipboardEntry[], lineEnding: ClipboardLineEnding, tokens: SemanticTokenSource | undefined, ownerDocument: Document): string {
	const included = entries.filter(entry => entry.text.length > 0);
	const contents = included.map(entry => renderClipboardEntry(entry, tokens, ownerDocument));
	const separators = included.map((entry, index) => index === 0 || included[index - 1]!.pasteMode === EditorClipboardPasteMode.Line ? '' : '\n');
	return `<pre><code>${toExternalLineEndings(contents.map((content, index) => `${separators[index]}${content}`).join(''), lineEnding)}</code></pre>`;
}

function renderClipboardEntry(entry: EditorClipboardEntry, tokens: SemanticTokenSource | undefined, ownerDocument: Document): string {
	if (!tokens) return escapeHtml(entry.text);
	try {
		const model = tokens.textModel;
		const endOffset = model.offsetAt(entry.sourceRange.getEndPosition());
		const exactStartOffset = endOffset - entry.text.length;
		if (exactStartOffset >= 0 && model.getText().slice(exactStartOffset, endOffset) === entry.text) {
			return renderTokenizedRange(tokens, model.positionAt(exactStartOffset), entry.sourceRange.getEndPosition(), ownerDocument);
		}
		if (entry.pasteMode !== EditorClipboardPasteMode.Line || !entry.text.endsWith('\n')) return escapeHtml(entry.text);
		const lineText = entry.text.slice(0, -1);
		const contentEndOffset = model.getText()[endOffset - 1] === '\n' ? endOffset - 1 : endOffset;
		const contentStartOffset = contentEndOffset - lineText.length;
		if (contentStartOffset < 0 || model.getText().slice(contentStartOffset, contentEndOffset) !== lineText) return escapeHtml(entry.text);
		return `${renderTokenizedRange(tokens, model.positionAt(contentStartOffset), model.positionAt(contentEndOffset), ownerDocument)}\n`;
	} catch {
		return escapeHtml(entry.text);
	}
}

function renderTokenizedRange(tokens: SemanticTokenSource, start: Position, end: Position, ownerDocument: Document): string {
	const parts: string[] = [];
	const model = tokens.textModel;
	const colors = resolveTokenColors(ownerDocument);
	for (let lineNumber = start.lineNumber; lineNumber <= end.lineNumber; lineNumber += 1) {
		const lineText = model.getLineContent(lineNumber);
		const startColumn = lineNumber === start.lineNumber ? start.column - 1 : 0;
		const endColumn = lineNumber === end.lineNumber ? end.column - 1 : lineText.length;
		parts.push(renderTokenizedLine(lineText, startColumn, endColumn, tokens.getLineTokens(lineNumber - 1), colors));
		if (lineNumber < end.lineNumber) parts.push('\n');
	}
	return parts.join('');
}

function renderTokenizedLine(lineText: string, startColumn: number, endColumn: number, tokens: ReturnType<SemanticTokenSource['getLineTokens']>, colors: ReadonlyMap<SemanticTokenPresentation, string>): string {
	let column = startColumn;
	const parts: string[] = [];
	for (const token of tokens) {
		const tokenStart = Math.max(startColumn, token.startColumn);
		const tokenEnd = Math.min(endColumn, token.endColumn);
		if (tokenEnd <= tokenStart) continue;
		if (column < tokenStart) parts.push(escapeHtml(lineText.slice(column, tokenStart)));
		const color = token.syntaxPresentation?.foreground ?? (token.presentation ? colors.get(token.presentation) : undefined);
		const style = color ? ` style="color: ${escapeHtml(color)}"` : '';
		const modifiers = token.modifiers?.join(' ') ?? '';
		parts.push(`<span class="stanza-editor-token${token.presentation ? ` ${token.presentation}` : ''}${modifiers ? ` ${modifiers}` : ''}"${style}>${escapeHtml(lineText.slice(tokenStart, tokenEnd))}</span>`);
		column = tokenEnd;
	}
	if (column < endColumn) parts.push(escapeHtml(lineText.slice(column, endColumn)));
	return parts.join('');
}

function resolveTokenColors(ownerDocument: Document): ReadonlyMap<SemanticTokenPresentation, string> {
	const view = ownerDocument.defaultView;
	if (!view) return new Map();
	const style = view.getComputedStyle(ownerDocument.documentElement);
	const colors = new Map<SemanticTokenPresentation, string>();
	for (const [presentation, variable] of TOKEN_COLOR_VARIABLES) {
		const color = style.getPropertyValue(variable).trim();
		if (color.length > 0) colors.set(presentation, color);
	}
	return colors;
}

const TOKEN_COLOR_VARIABLES = new Map<SemanticTokenPresentation, string>([
	[SemanticTokenPresentation.Comment, '--zeta-editor-token-comment-foreground'],
	[SemanticTokenPresentation.Keyword, '--zeta-editor-token-keyword-foreground'],
	[SemanticTokenPresentation.String, '--zeta-editor-token-string-foreground'],
	[SemanticTokenPresentation.Number, '--zeta-editor-token-number-foreground'],
	[SemanticTokenPresentation.Regexp, '--zeta-editor-token-regexp-foreground'],
	[SemanticTokenPresentation.Type, '--zeta-editor-token-type-foreground'],
	[SemanticTokenPresentation.Function, '--zeta-editor-token-function-foreground'],
	[SemanticTokenPresentation.Variable, '--zeta-editor-token-variable-foreground'],
	[SemanticTokenPresentation.Operator, '--zeta-editor-token-operator-foreground'],
]);

function toExternalLineEndings(text: string, lineEnding: ClipboardLineEnding): string {
	return lineEnding === ClipboardLineEnding.LF ? text : text.replaceAll('\n', ClipboardLineEnding.CRLF);
}

function escapeHtml(text: string): string {
	return text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;').replaceAll("'", '&#39;');
}

function readUriList(value: string): string | undefined {
	const entries = UriList.parse(value).map(entry => entry.trim()).filter(entry => entry.length > 0);
	return entries.length > 0 ? entries.join('\n') : undefined;
}

function createMetadataPasteCommand(model: TextModel, selections: readonly Selection[], data: EditorClipboardPasteData): EditorEditCommand {
	return data.modes.every(mode => mode === EditorClipboardPasteMode.Line) &&
		canPasteCompleteLines(selections)
		? TypeOperations.linePaste(model, selections, data.texts)
		: TypeOperations.distributedPaste(model, selections, data.texts);
}

function canPasteCompleteLines(selections: readonly Selection[]): boolean {
	return selections.every(selection => selection.isEmpty());
}

function readEmptySelectionPolicy(policy: EditorEmptySelectionClipboardPolicy | undefined): EditorEmptySelectionClipboardPolicy {
	const resolved = policy ?? EditorEmptySelectionClipboardPolicy.Line;
	if (!Object.values(EditorEmptySelectionClipboardPolicy).includes(resolved)) {
		throw new TypeError("Unknown Stanza empty-selection clipboard policy");
	}
	return resolved;
}

function selectionSetsEqual(left: readonly Selection[], right: readonly Selection[]): boolean {
	return 0 === 0 &&
		left.length === right.length &&
		left.every((selection, index) => {
			const expected = right[index]!;
			return Position.compare(selection.getSelectionStart(), expected.getSelectionStart()) === 0 &&
				Position.compare(selection.getPosition(), expected.getPosition()) === 0;
		});
}

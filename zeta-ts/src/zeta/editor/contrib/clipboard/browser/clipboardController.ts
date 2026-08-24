import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { isWindows } from "../../../../base/common/platform.js";
import { EditorClipboardPasteMode, EditorEmptySelectionClipboardPolicy, getEditorClipboardEntries, type EditorClipboardEntry } from "../common/clipboard.js";
import { createClipboardCutCommand } from "../../../common/cursor/cursorDeleteOperations.js";
import { createDistributedPasteTextCommand, createLinePasteCommand, createPasteTextCommand } from "../../../common/cursor/cursorTypeOperations.js";
import { type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextSelectionSet } from "../../../common/core/selection.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type SemanticTokenSource } from "../../../browser/viewparts/semanticTokens/semanticTokenPresentation.js";
import { createStanzaSyntaxClipboardHtml } from "./syntaxClipboardHtml.js";
import { TEXT_FILE_TRANSFER_MAX_BYTES, selectTextFileTransfer } from "../../dropOrPasteInto/browser/textFileTransfer.js";
import { captureStanzaClipboardTextTransfer, normalizeStanzaClipboardPasteProviders, provideStanzaClipboardPaste, type ClipboardPasteProvider } from "./clipboardPasteProvider.js";
import { createStanzaBrowserClipboardSystemTextReader, type ClipboardSystemTextReader } from "./clipboardSystemText.js";
import { createStanzaBrowserClipboardRichTextReader, createStanzaBrowserClipboardRichTextWriter, type ClipboardRichTextItem, type ClipboardRichTextReader, type ClipboardRichTextWriter } from "./clipboardRichText.js";
import { registerTextInputClipboardFactory } from "../../../browser/input/textInputController.js";
import { UriListPasteProvider } from "./clipboardPasteProvider.js";

export const EDITOR_CLIPBOARD_MIME = "application/x-stanza-editor";
export const EDITOR_HTML_CLIPBOARD_MIME = "text/html";

export enum ClipboardLineEnding {
	LF = "\n",
	CRLF = "\r\n",
}

export interface ClipboardControllerOptions {
	readonly lineEnding?: ClipboardLineEnding;
	readonly emptySelectionPolicy?: EditorEmptySelectionClipboardPolicy;
	/** Optional current token projection used only for portable HTML copy output. */
	readonly semanticTokens?: SemanticTokenSource;
	/** Rejects cut and paste while another input adapter owns a protected edit. */
	readonly isEditingAllowed?: () => boolean;
	/** Ordered local providers for declared non-plain clipboard representations. */
	readonly pasteProviders?: readonly ClipboardPasteProvider[];
	/**
	 * Optional Async Clipboard plain-text fallback. It is used only when the
	 * native paste event has no textual, metadata, file, or provider payload.
	 */
	readonly systemTextReader?: ClipboardSystemTextReader;
	/** Optional rich Async Clipboard fallback, used before the plain-text fallback. */
	readonly richTextReader?: ClipboardRichTextReader;
	/** Optional rich Async Clipboard writer, used only without event clipboard data. */
	readonly richTextWriter?: ClipboardRichTextWriter;
}

interface ClipboardMetadata {
	readonly version: 2;
	readonly selectionTexts: readonly string[];
	readonly pasteModes: readonly EditorClipboardPasteMode[];
}

interface ClipboardPasteData {
	readonly texts: readonly string[];
	readonly modes: readonly EditorClipboardPasteMode[];
}

/**
 * Routes native clipboard events through Stanza's selection-aware commands.
 */
export class ClipboardController extends DisposableOwner {
	private readonly lineEnding: ClipboardLineEnding;
	private readonly emptySelectionPolicy: EditorEmptySelectionClipboardPolicy;
	private readonly semanticTokens: SemanticTokenSource | undefined;
	private readonly isEditingAllowed: () => boolean;
	private readonly pasteProviders: readonly ClipboardPasteProvider[];
	private readonly systemTextReader: ClipboardSystemTextReader | undefined;
	private readonly richTextReader: ClipboardRichTextReader | undefined;
	private readonly richTextWriter: ClipboardRichTextWriter | undefined;
	private asynchronousPasteRequest = 0;

	constructor(
		private readonly element: HTMLTextAreaElement,
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: ClipboardControllerOptions = {},
	) {
		super();
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
		if (options.systemTextReader !== undefined && typeof options.systemTextReader.readText !== "function") {
			this.dispose();
			throw new TypeError("Stanza clipboard system text reader must provide readText");
		}
		if (options.richTextReader !== undefined && typeof options.richTextReader.readText !== "function") {
			this.dispose();
			throw new TypeError("Stanza clipboard rich text reader must provide readText");
		}
		if (options.richTextWriter !== undefined && typeof options.richTextWriter.writeText !== "function") {
			this.dispose();
			throw new TypeError("Stanza clipboard rich text writer must provide writeText");
		}
		this.lineEnding = readLineEnding(options.lineEnding);
		this.emptySelectionPolicy = readEmptySelectionPolicy(
			options.emptySelectionPolicy,
		);
		this.semanticTokens = options.semanticTokens;
		this.isEditingAllowed = options.isEditingAllowed ?? (() => true);
		this.pasteProviders = normalizeStanzaClipboardPasteProviders(options.pasteProviders);
		this.systemTextReader = options.systemTextReader ?? createStanzaBrowserClipboardSystemTextReader(element.ownerDocument);
		this.richTextReader = options.richTextReader ?? createStanzaBrowserClipboardRichTextReader(element.ownerDocument);
		this.richTextWriter = options.richTextWriter ?? createStanzaBrowserClipboardRichTextWriter(element.ownerDocument);
		this.defer(() => {
			this.asynchronousPasteRequest += 1;
		});
		this.own(addDisposableListener<ClipboardEvent>(
			element,
			"copy",
			event => this.handleCopy(event),
		));
		this.own(addDisposableListener<ClipboardEvent>(
			element,
			"cut",
			event => this.handleCut(event),
		));
		this.own(addDisposableListener<ClipboardEvent>(
			element,
			"paste",
			event => this.handlePaste(event),
		));
	}

	private handleCopy(event: ClipboardEvent): void {
		if (event.defaultPrevented) return;
		const entries = getEditorClipboardEntries(
			this.viewport.textModel,
			this.selectionController.selections,
			this.emptySelectionPolicy,
		);
		if (this.writeClipboard(event.clipboardData, entries)) {
			stopEvent(event);
			return;
		}
		this.writeRichSystemClipboard(event, entries);
	}

	private handleCut(event: ClipboardEvent): void {
		if (event.defaultPrevented) return;
		if (!this.isEditingAllowed()) {
			stopEvent(event);
			return;
		}
		const entries = getEditorClipboardEntries(
			this.viewport.textModel,
			this.selectionController.selections,
			this.emptySelectionPolicy,
		);
		if (this.writeClipboard(event.clipboardData, entries)) {
			stopEvent(event);
			this.executeCut();
			return;
		}
		this.writeRichSystemClipboard(event, entries, true);
	}

	private handlePaste(event: ClipboardEvent): void {
		const nativeClipboard = event.clipboardData;
		if (event.defaultPrevented || !nativeClipboard) return;
		if (!this.isEditingAllowed()) {
			stopEvent(event);
			return;
		}
		const text = readClipboardText(nativeClipboard, this.element.ownerDocument);
		const clipboardData = readClipboardMetadata(
			nativeClipboard,
			this.selectionController.selections.selections.length,
		);
		if (text.length === 0 && !clipboardData?.texts.some(value => value.length > 0)) {
			if (this.pasteProviders.some(provider => provider.mimeTypes.some(type => Array.from(nativeClipboard.types).includes(type)))) {
				this.pasteProvidedText(event);
				return;
			}
			if (this.pasteTextFile(event)) return;
			if (this.pasteRichSystemText(event)) return;
			this.pasteSystemText(event);
			return;
		}
		const command = clipboardData
			? createMetadataPasteCommand(
				this.viewport.textModel,
				this.selectionController.selections,
				clipboardData,
			)
			: createPasteTextCommand(
				this.viewport.textModel,
				this.selectionController.selections,
				text,
			);
		stopEvent(event);
		this.selectionController.execute(command);
		this.afterEdit();
	}

	private pasteTextFile(event: ClipboardEvent): boolean {
		const file = selectTextFileTransfer(event.clipboardData?.files ?? []);
		if (!file) return false;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		stopEvent(event);
		void file.text().then(text => {
			if (
				this.isDisposed ||
				request !== this.asynchronousPasteRequest ||
				text.length > TEXT_FILE_TRANSFER_MAX_BYTES ||
				!this.isEditingAllowed() ||
				model.version !== expectedVersion ||
				!selectionSetsEqual(this.selectionController.selections, expectedSelections)
			) {
				return;
			}
			this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
			this.afterEdit();
		}).catch(() => {
			// The host supplied the file, but it could not be decoded as text.
		});
		return true;
	}

	private pasteProvidedText(event: ClipboardEvent): void {
		const clipboardData = event.clipboardData;
		if (!clipboardData) return;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		const transfer = captureStanzaClipboardTextTransfer(clipboardData);
		stopEvent(event);
		void provideStanzaClipboardPaste(this.pasteProviders, transfer).then(text => {
			if (
				text === undefined ||
				this.isDisposed ||
				request !== this.asynchronousPasteRequest ||
				!this.isEditingAllowed() ||
				model.version !== expectedVersion ||
				!selectionSetsEqual(this.selectionController.selections, expectedSelections)
			) {
				return;
			}
			this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
			this.afterEdit();
		}).catch(() => {
			// A provider is optional; invalid or failed output must not mutate the model.
		});
	}

	private pasteSystemText(event: ClipboardEvent): boolean {
		const reader = this.systemTextReader;
		if (!reader) return false;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		stopEvent(event);
		void Promise.resolve(reader.readText()).then(text => {
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
			this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
			this.afterEdit();
		}).catch(() => {
			// Permission failures and unavailable system text must leave the model unchanged.
		});
		return true;
	}

	private pasteRichSystemText(event: ClipboardEvent): boolean {
		const reader = this.richTextReader;
		if (!reader) return false;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		stopEvent(event);
		void Promise.resolve(reader.readText()).then(item => {
			const text = item?.plainText ?? (item?.html ? readEditorHtmlText(item.html, this.element.ownerDocument) : "");
			if (text.length === 0 || this.isDisposed || request !== this.asynchronousPasteRequest || !this.isEditingAllowed() || model.version !== expectedVersion || !selectionSetsEqual(this.selectionController.selections, expectedSelections)) return;
			this.selectionController.execute(createPasteTextCommand(model, expectedSelections, text));
			this.afterEdit();
		}).catch(() => {
			// Permission and representation failures leave the model unchanged.
		});
		return true;
	}

	private writeRichSystemClipboard(event: ClipboardEvent, entries: readonly EditorClipboardEntry[], cut = false): boolean {
		const writer = this.richTextWriter;
		if (!writer || !entries.some(entry => entry.text.length > 0)) return false;
		const model = this.viewport.textModel;
		const expectedVersion = model.version;
		const expectedSelections = this.selectionController.selections;
		const request = ++this.asynchronousPasteRequest;
		const payload = this.createClipboardPayload(entries);
		stopEvent(event);
		void Promise.resolve(writer.writeText(payload)).then(() => {
			if (!cut || this.isDisposed || request !== this.asynchronousPasteRequest || !this.isEditingAllowed() || model.version !== expectedVersion || !selectionSetsEqual(this.selectionController.selections, expectedSelections)) return;
			this.executeCut();
		}).catch(() => {
			// Permission failures must never mutate the model, especially for cut.
		});
		return true;
	}

	private executeCut(): void {
		this.selectionController.execute(createClipboardCutCommand(
			this.viewport.textModel,
			this.selectionController.selections,
			this.emptySelectionPolicy,
		));
		this.afterEdit();
	}

	private writeClipboard(clipboardData: DataTransfer | null, entries: readonly EditorClipboardEntry[]): boolean {
		if (!clipboardData) return false;
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
		const metadata: ClipboardMetadata = {
			version: 2,
			selectionTexts: entries.map(entry => entry.text),
			pasteModes: entries.map(entry => entry.pasteMode),
		};
		try {
			clipboardData.setData(
				EDITOR_CLIPBOARD_MIME,
				JSON.stringify(metadata),
			);
		} catch {
			// Plain text remains portable when a browser rejects custom MIME data.
		}
		try {
			clipboardData.setData(EDITOR_HTML_CLIPBOARD_MIME, payload.html);
		} catch {
			// Plain text remains authoritative when a browser rejects HTML clipboard data.
		}
		return true;
	}

	private createClipboardPayload(entries: readonly EditorClipboardEntry[]): Required<ClipboardRichTextItem> {
		return Object.freeze({
			plainText: joinClipboardEntries(entries, this.lineEnding),
			html: createStanzaSyntaxClipboardHtml(
				entries,
				this.lineEnding,
				this.semanticTokens,
				this.element.ownerDocument,
			),
		});
	}

	private afterEdit(): void {
		this.element.value = "";
		this.viewport.revealPosition(
			this.selectionController.selections.primary.active,
		);
	}
}

registerTextInputClipboardFactory((element, viewport, selections, options, isEditingAllowed) => {
	const clipboardOptions = options as ClipboardControllerOptions;
	return new ClipboardController(element, viewport, selections, {
		...clipboardOptions,
		isEditingAllowed: () => isEditingAllowed() && (clipboardOptions.isEditingAllowed?.() ?? true),
		pasteProviders: [UriListPasteProvider, ...(clipboardOptions.pasteProviders ?? [])],
	});
});

function createMetadataPasteCommand(model: TextModel, selections: TextSelectionSet, data: ClipboardPasteData): EditorEditCommand {
	return data.modes.every(mode => mode === EditorClipboardPasteMode.Line) &&
		canPasteCompleteLines(selections)
		? createLinePasteCommand(model, selections, data.texts)
		: createDistributedPasteTextCommand(model, selections, data.texts);
}

function canPasteCompleteLines(selections: TextSelectionSet): boolean {
	return selections.selections.every(selection => selection.collapsed);
}

function readLineEnding(lineEnding: ClipboardLineEnding | undefined): ClipboardLineEnding {
	const resolved = lineEnding ?? (
		isWindows ? ClipboardLineEnding.CRLF : ClipboardLineEnding.LF
	);
	if (!Object.values(ClipboardLineEnding).includes(resolved)) {
		throw new TypeError("Unknown Stanza clipboard line ending");
	}
	return resolved;
}

function readEmptySelectionPolicy(policy: EditorEmptySelectionClipboardPolicy | undefined): EditorEmptySelectionClipboardPolicy {
	const resolved = policy ?? EditorEmptySelectionClipboardPolicy.Line;
	if (!Object.values(EditorEmptySelectionClipboardPolicy).includes(resolved)) {
		throw new TypeError("Unknown Stanza empty-selection clipboard policy");
	}
	return resolved;
}

function joinClipboardEntries(entries: readonly EditorClipboardEntry[], lineEnding: ClipboardLineEnding): string {
	const included = entries.filter(entry => entry.text.length > 0);
	let result = "";
	let previousMode: EditorClipboardPasteMode | undefined;
	for (const entry of included) {
		if (
			result.length > 0 &&
			previousMode !== EditorClipboardPasteMode.Line
		) {
			result += lineEnding;
		}
		result += toExternalLineEndings(entry.text, lineEnding);
		previousMode = entry.pasteMode;
	}
	return result;
}

function toExternalLineEndings(text: string, lineEnding: ClipboardLineEnding): string {
	return lineEnding === ClipboardLineEnding.LF
		? text
		: text.replaceAll("\n", ClipboardLineEnding.CRLF);
}

function readClipboardText(clipboardData: DataTransfer, ownerDocument: Document): string {
	try {
		const text = clipboardData.getData("text/plain");
		if (text.length > 0) return text;
	} catch {
		// A browser may expose only a rich clipboard representation.
	}
	try {
		return readEditorHtmlText(clipboardData.getData(EDITOR_HTML_CLIPBOARD_MIME), ownerDocument);
	} catch {
		return "";
	}
}

function selectionSetsEqual(left: TextSelectionSet, right: TextSelectionSet): boolean {
	return left.primaryIndex === right.primaryIndex &&
		left.selections.length === right.selections.length &&
		left.selections.every((selection, index) => {
			const expected = right.selections[index]!;
			return selection.anchor.compareTo(expected.anchor) === 0 &&
				selection.active.compareTo(expected.active) === 0;
		});
}

/** Reduces untrusted HTML to inert deterministic text for Stanza paste and drop paths. */
export function readEditorHtmlText(html: string, ownerDocument: Document): string {
	if (html.length === 0) return "";
	const template = h(ownerDocument, "template");
	template.innerHTML = html;
	const parts: string[] = [];
	appendHtmlClipboardText(template.content, parts);
	return parts.join("").replaceAll("\u00a0", " ").replace(/\n{3,}/g, "\n\n").replace(/^\n|\n$/g, "");
}

function appendHtmlClipboardText(node: Node, parts: string[]): void {
	if (node.nodeType === node.TEXT_NODE) {
		parts.push(node.textContent ?? "");
		return;
	}
	if (node.nodeType !== node.ELEMENT_NODE && node.nodeType !== node.DOCUMENT_FRAGMENT_NODE) return;
	const element = node.nodeType === node.ELEMENT_NODE ? node as HTMLElement : undefined;
	if (element && (element.localName === "script" || element.localName === "style" || element.localName === "noscript")) return;
	if (element?.localName === "br") {
		appendLineBreak(parts);
		return;
	}
	const block = element !== undefined && HTML_CLIPBOARD_BLOCK_ELEMENTS.has(element.localName);
	if (block) appendLineBreak(parts);
	for (const child of node.childNodes) appendHtmlClipboardText(child, parts);
	if (block) appendLineBreak(parts);
}

function appendLineBreak(parts: string[]): void {
	if (parts.length === 0 || parts.at(-1) !== "\n") parts.push("\n");
}

const HTML_CLIPBOARD_BLOCK_ELEMENTS = new Set([
	"address", "article", "aside", "blockquote", "div", "dl", "dt", "dd", "fieldset", "figcaption",
	"figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "li",
	"main", "nav", "ol", "p", "section", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "ul",
]);

function readClipboardMetadata(clipboardData: DataTransfer, selectionCount: number): ClipboardPasteData | undefined {
	let parsed: unknown;
	try {
		const raw = clipboardData.getData(EDITOR_CLIPBOARD_MIME);
		if (!raw) return undefined;
		parsed = JSON.parse(raw);
	} catch {
		return undefined;
	}
	if (
		typeof parsed !== "object" ||
		parsed === null ||
		!("version" in parsed) ||
		(parsed.version !== 1 && parsed.version !== 2) ||
		!("selectionTexts" in parsed) ||
		!Array.isArray(parsed.selectionTexts) ||
		parsed.selectionTexts.length !== selectionCount ||
		parsed.selectionTexts.some(text => typeof text !== "string")
	) {
		return undefined;
	}
	const texts = parsed.selectionTexts as string[];
	let modes = parsed.version === 2 &&
		"pasteModes" in parsed &&
		Array.isArray(parsed.pasteModes) &&
		parsed.pasteModes.length === selectionCount &&
		parsed.pasteModes.every(mode =>
			Object.values(EditorClipboardPasteMode).includes(mode)
		)
		? parsed.pasteModes as EditorClipboardPasteMode[]
		: texts.map(() => EditorClipboardPasteMode.Selection);
	if (modes.some((mode, index) =>
		mode === EditorClipboardPasteMode.Line &&
		!texts[index]!.endsWith("\n")
	)) {
		modes = texts.map(() => EditorClipboardPasteMode.Selection);
	}
	return Object.freeze({
		texts: Object.freeze([...texts]),
		modes: Object.freeze([...modes]),
	});
}

import "./symbolIcons.css";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { Range } from "../../../common/core/range.js";
import { TextDecorationCollection, type TextDecorationId } from "../../../common/model/decorationCollection.js";
import { type TextModel } from "../../../common/model/textModel.js";

import { createStanzaDecorationSource, DecorationPresentation, type DecorationPresentationResolution, type DecorationSource, type OwnedDecorationSource } from "../../../browser/viewParts/decorations/decorations.js";
import { type DocumentSymbolService, type LanguageDocumentSymbol } from "../../documentSymbols/common/languageDocumentSymbols.js";
import { TrackedRangeStickiness } from '../../../common/model.js';

const SYMBOL_ICON_DECORATION_OWNER = "symbol-icons";

interface SymbolIconMetadata {
	readonly kind: LanguageDocumentSymbol["kind"];
	readonly name: string;
	readonly detail: string | undefined;
}

/** Resolves document symbols into the shared line-decoration Part. */
export class SymbolIconsController extends Disposable implements OwnedDecorationSource {
	private readonly collection: TextDecorationCollection<SymbolIconMetadata>;
	private readonly source: DecorationSource;
	private decorationIds: readonly TextDecorationId[] = Object.freeze([]);
	private request: AbortController | undefined;

	public readonly onDidChange;
	public readonly glyphMarginLanes;
	public readonly linesDecorationLanes;

	constructor(
		model: TextModel,
		private readonly service: DocumentSymbolService,
		private readonly languageId: string,
		private readonly onError: (error: unknown) => void = error => console.error("Stanza symbol icons failed", error),
	) {
		super();
		if (service.textModel !== model) throw new TypeError("Stanza symbol icon dependencies must share a text model");
		this.collection = this._register(new TextDecorationCollection(model));
		this.source = createStanzaDecorationSource(
			this.collection,
			decoration => symbolIconDecoration(decoration.metadata),
			undefined,
			{ linesDecorationLanes: [{ owner: SYMBOL_ICON_DECORATION_OWNER, width: 20 }] },
		);
		this.onDidChange = this.source.onDidChange;
		this.glyphMarginLanes = this.source.glyphMarginLanes;
		this.linesDecorationLanes = this.source.linesDecorationLanes;
		this._register(model.onDidChange(() => void this.refresh()));
		this._register(toDisposable(() => this.request?.abort()));
		void this.refresh();
	}

	public get decorations() {
		return this.source.decorations;
	}

	private async refresh(): Promise<void> {
		this.request?.abort();
		const request = this.request = new AbortController();
		try {
			const symbols = await this.service.provideDocumentSymbols(this.languageId, request.signal);
			if (request.signal.aborted || request !== this.request) return;
			const seenLines = new Set<number>();
			this.decorationIds = this.collection.deltaDecorations(
				this.decorationIds,
				flatten(symbols).flatMap(symbol => {
					const lineNumber = symbol.selectionRange.startLineNumber;
					const lineIndex = lineNumber - 1;
					if (seenLines.has(lineIndex)) return [];
					seenLines.add(lineIndex);
					return [{
						range: Range.fromPositions({ lineNumber, column: 1 }),
						stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
						metadata: Object.freeze({
							kind: symbol.kind,
							name: symbol.name,
							detail: symbol.detail,
						}),
					}];
				}),
			);
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}
}

function symbolIconDecoration(metadata: SymbolIconMetadata): DecorationPresentationResolution {
	return Object.freeze({
		presentation: DecorationPresentation.LineDecoration,
		linesDecoration: {
			owner: SYMBOL_ICON_DECORATION_OWNER,
			className: `stanza-editor-symbol-icon ${symbolKindClass(metadata.kind)}`,
			tooltip: metadata.detail ? `${metadata.name}: ${metadata.detail}` : metadata.name,
		},
		overviewRuler: false,
		minimap: false,
	});
}

function flatten(symbols: readonly LanguageDocumentSymbol[]): readonly LanguageDocumentSymbol[] {
	return symbols.flatMap(symbol => [symbol, ...flatten(symbol.children ?? [])]);
}

function symbolKindClass(kind: LanguageDocumentSymbol["kind"]): string {
	const value = String(kind).toLowerCase();
	if (value.includes("class") || value === "5") return "class";
	if (value.includes("function") || value === "12") return "function";
	if (value.includes("property") || value === "10") return "property";
	return "symbol";
}

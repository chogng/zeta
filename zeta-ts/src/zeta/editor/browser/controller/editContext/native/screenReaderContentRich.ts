import { fragment as createFragment, h, text as createText } from "../../../../../base/browser/dom.js";
import { type TextModel } from "../../../../common/model/textModel.js";
import { projectStanzaSemanticTokenLine, type BracketColorizationSource, type BracketColorizationSpan, type ResolvedSemanticToken, type SemanticTokenSource } from "../../../viewparts/viewLines/viewLine.js";
import { type ScreenReaderContentState } from "./screenReaderUtils.js";
import { SimpleScreenReaderContent } from "./screenReaderContentSimple.js";

export interface RichScreenReaderContentOptions {
	readonly model: TextModel;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
}

/**
 * Line-structured screen-reader projection with the same token boundaries as
 * the visible Stanza text renderer.
 *
 * The text remains exact and bounded, while token and bracket spans provide a
 * stable rich DOM for assistive technology and keep DOM selection offsets in
 * the same UTF-16 coordinate space as the model.
 */
export class RichScreenReaderContent extends SimpleScreenReaderContent {
	constructor(
		host: HTMLElement,
		private readonly options: RichScreenReaderContentOptions,
	) {
		super(host);
		if (options.semanticTokenSource && options.semanticTokenSource.textModel !== options.model) {
			throw new TypeError("Native rich screen-reader semantic tokens must share the text model");
		}
		if (options.bracketColorizationSource && options.bracketColorizationSource.textModel !== options.model) {
			throw new TypeError("Native rich screen-reader brackets must share the text model");
		}
	}

	protected override renderText(text: string, state: ScreenReaderContentState): void {
		const ownerDocument = this.element.ownerDocument;
		const fragment = createFragment(ownerDocument);
		for (const [index, segment] of state.segments.entries()) {
			if (index > 0) {
				fragment.append(createText(ownerDocument, text.slice(
					state.segments[index - 1]!.contentEndOffset,
					segment.contentStartOffset,
				)));
			}
			const segmentText = text.slice(segment.contentStartOffset, segment.contentEndOffset);
			const startPosition = this.options.model.positionAt(segment.modelStartOffset);
			renderSegment(
				fragment,
				segmentText,
				startPosition.lineNumber - 1,
				startPosition.column - 1,
				this.options,
			);
		}
		this.element.replaceChildren(fragment);
	}
}

function renderSegment(
	fragment: DocumentFragment,
	text: string,
	startLineIndex: number,
	startColumn: number,
	options: RichScreenReaderContentOptions,
): void {
	const ownerDocument = fragment.ownerDocument;
	const lines = text.split("\n");
	const lineCount = text.endsWith("\n") ? lines.length - 1 : lines.length;
	let lineIndex = startLineIndex;
	let lineStartColumn = startColumn;
	for (let index = 0; index < lineCount; index += 1) {
		const lineText = lines[index]!;
		const lineElement = h(ownerDocument, "span");
		lineElement.dataset.lineIndex = String(lineIndex);
		const lineEndColumn = lineStartColumn + lineText.length;
		projectStanzaSemanticTokenLine(
			lineElement,
			lineText,
			clipSemanticTokens(options.semanticTokenSource?.getLineTokens(lineIndex) ?? [], lineStartColumn, lineEndColumn),
			clipBracketColorizations(options.bracketColorizationSource?.getLineBrackets(lineIndex) ?? [], lineStartColumn, lineEndColumn),
		);
		if (!lineElement.firstChild) lineElement.append(createText(ownerDocument, ""));
		fragment.append(lineElement);
		if (index < lines.length - 1) fragment.append(createText(ownerDocument, "\n"));
		lineIndex += 1;
		lineStartColumn = 0;
	}
}

function clipSemanticTokens(
	tokens: readonly ResolvedSemanticToken[],
	startColumn: number,
	endColumn: number,
): readonly ResolvedSemanticToken[] {
	return Object.freeze(tokens.flatMap(token => {
		const start = Math.max(token.startColumn, startColumn);
		const end = Math.min(token.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({
			startColumn: start - startColumn,
			endColumn: end - startColumn,
			...(token.presentation === undefined ? {} : { presentation: token.presentation }),
			...(token.modifiers === undefined ? {} : { modifiers: token.modifiers }),
			...(token.syntaxPresentation === undefined ? {} : { syntaxPresentation: token.syntaxPresentation }),
		})];
	}));
}

function clipBracketColorizations(
	brackets: readonly BracketColorizationSpan[],
	startColumn: number,
	endColumn: number,
): readonly BracketColorizationSpan[] {
	return Object.freeze(brackets.flatMap(bracket => {
		const start = Math.max(bracket.startColumn, startColumn);
		const end = Math.min(bracket.endColumn, endColumn);
		if (end <= start) return [];
		return [Object.freeze({
			startColumn: start - startColumn,
			endColumn: end - startColumn,
			level: bracket.level,
		})];
	}));
}

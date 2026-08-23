import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type LanguageStructuralBracketSource } from "../../../common/languages/languageLexicalContext.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageBracketColorization {
	readonly startColumn: number;
	readonly endColumn: number;
	readonly level: number;
}

interface BracketStackEntry {
	readonly closingToken: string;
	readonly level: number;
}

interface CachedColorizedLine {
	readonly colorizations: readonly LanguageBracketColorization[];
	readonly outputStack: readonly BracketStackEntry[];
}

/** Caches lexical bracket nesting colors while retaining no renderer state. */
export class LanguageBracketColorizationIndex extends DisposableOwner {
	private cachedLines: CachedColorizedLine[] = [];
	private disposed = false;

	constructor(readonly textModel: TextModel, private readonly brackets: LanguageStructuralBracketSource, private readonly colorCount = 6) {
		super();
		if (brackets.textModel !== textModel) {
			this.dispose();
			throw new TypeError("Bracket colorization and lexical context must share one text model");
		}
		if (!Number.isSafeInteger(colorCount) || colorCount < 1) {
			this.dispose();
			throw new RangeError("Bracket color count must be a positive safe integer");
		}
		this.own(textModel.onDidChange(() => {
			this.cachedLines = [];
		}));
		this.defer(() => {
			this.disposed = true;
			this.cachedLines = [];
		});
	}

	getLineColorizations(lineIndex: number): readonly LanguageBracketColorization[] {
		this.ensureAlive();
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.textModel.lineCount) {
			throw new RangeError("Bracket colorization line is outside the text model");
		}
		this.ensureLine(lineIndex);
		return this.cachedLines[lineIndex]!.colorizations;
	}

	private ensureLine(lineIndex: number): void {
		while (this.cachedLines.length <= lineIndex) {
			const currentLineIndex = this.cachedLines.length;
			const inputStack = this.cachedLines.at(-1)?.outputStack ?? [];
			const stack = [...inputStack];
			const colorizations: LanguageBracketColorization[] = [];
			for (const event of this.brackets.getStructuralBracketEvents(currentLineIndex)) {
				if (event.action === "open") {
					const level = stack.length % this.colorCount + 1;
					stack.push(Object.freeze({ closingToken: event.matchingToken, level }));
					colorizations.push(Object.freeze({ startColumn: event.startColumn, endColumn: event.endColumn, level }));
					continue;
				}
				const opening = stack.at(-1);
				if (!opening || opening.closingToken !== event.token) continue;
				stack.pop();
				colorizations.push(Object.freeze({ startColumn: event.startColumn, endColumn: event.endColumn, level: opening.level }));
			}
			this.cachedLines.push(Object.freeze({
				colorizations: Object.freeze(colorizations),
				outputStack: Object.freeze(stack),
			}));
		}
	}

	private ensureAlive(): void {
		if (this.disposed) throw new ReferenceError("LanguageBracketColorizationIndex is already disposed");
	}
}

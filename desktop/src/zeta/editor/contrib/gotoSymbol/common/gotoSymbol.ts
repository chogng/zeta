import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextPosition, type TextRange } from "../../../common/core/text.js";
import { type LanguageDocumentSymbol, DocumentSymbolService } from "../../documentSymbols/common/documentSymbols.js";

export interface LanguageSymbolMatch {
  readonly symbol: LanguageDocumentSymbol;
  readonly position: TextPosition;
  readonly score: number;
}

/** Queries document symbols without coupling quick access UI to provider transport. */
export class GotoSymbolService extends DisposableOwner {
  constructor(private readonly documentSymbols: DocumentSymbolService) {
    super();
    this.own(documentSymbols);
  }

  async query(languageId: string, query: string, signal: AbortSignal = new AbortController().signal): Promise<readonly LanguageSymbolMatch[]> {
    const symbols = await this.documentSymbols.provideDocumentSymbols(languageId, signal);
    const normalizedQuery = query.trim().toLocaleLowerCase();
    const matches: LanguageSymbolMatch[] = [];
    for (const symbol of flattenSymbols(symbols)) {
      const score = symbol.name.toLocaleLowerCase().includes(normalizedQuery) ? symbol.name.toLocaleLowerCase() === normalizedQuery ? 2 : 1 : 0;
      if (normalizedQuery.length > 0 && score === 0) continue;
      matches.push(Object.freeze({ symbol, position: symbol.selectionRange.start, score }));
    }
    matches.sort((left, right) => right.score - left.score || left.position.compareTo(right.position));
    return Object.freeze(matches);
  }
}

function flattenSymbols(symbols: readonly LanguageDocumentSymbol[]): readonly LanguageDocumentSymbol[] {
  const result: LanguageDocumentSymbol[] = [];
  const visit = (symbol: LanguageDocumentSymbol): void => {
    result.push(symbol);
    symbol.children?.forEach(visit);
  };
  symbols.forEach(visit);
  return result;
}

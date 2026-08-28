import { TextPosition, TextRange } from "../../../../editor/common/core/text.js";
import type { ILanguageFeaturesService } from '../../../../editor/common/services/languageFeatures.js';
import type { LanguageWorkspaceSymbol, LanguageWorkspaceSymbolProvider } from "../../../../editor/common/languages/workspaceSymbols.js";
import { workspaceResourceFromPath } from "../../../../platform/files/browser/fileService.js";
import type { ISymbolIndexService, SymbolIndexMatch } from "../../../../platform/symbolIndex/common/symbolIndexService.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";

const MAX_WORKSPACE_SYMBOL_RESULTS = 100;

/** Registers the local syntax declaration projection as one Workspace Symbol provider. */
export function registerSymbolIndexWorkspaceSymbolProvider(languageFeatures: ILanguageFeaturesService, symbols: ISymbolIndexService, workspace: IWorkspaceContextService) {
	return languageFeatures.workspaceSymbolProvider.register(new SymbolIndexWorkspaceSymbolProvider(symbols, workspace));
}

class SymbolIndexWorkspaceSymbolProvider implements LanguageWorkspaceSymbolProvider {
	readonly languageIds = Object.freeze(["*"]);
	readonly providerId = "zeta.localSymbolIndex.workspaceSymbols";

	constructor(private readonly symbols: ISymbolIndexService, private readonly workspace: IWorkspaceContextService) {}

	async provideWorkspaceSymbols(query: string, signal: AbortSignal): Promise<readonly LanguageWorkspaceSymbol[]> {
		const root = singleWorkspaceRoot(this.workspace);
		if (!root) return Object.freeze([]);
		const result = await this.symbols.search(query, MAX_WORKSPACE_SYMBOL_RESULTS, signal);
		return Object.freeze(result.matches.flatMap(match => {
			const resource = workspaceResourceFromPath(root, match.path);
			if (!resource) return [];
			return [Object.freeze({
				name: match.name,
				kind: match.kind,
				resource,
				range: textRange(match),
				...(match.containerName === undefined ? {} : { containerName: match.containerName }),
				data: Object.freeze({ source: "localSymbolIndex", score: match.score, sourceRevision: match.sourceRevision, matchedIndices: match.matchedIndices }),
			})];
		}));
	}
}

function singleWorkspaceRoot(workspace: IWorkspaceContextService) {
	const folders = workspace.getWorkspace().folders;
	return folders.length === 1 ? folders[0]?.uri : undefined;
}

function textRange(match: SymbolIndexMatch): TextRange {
	return TextRange.from(
		TextPosition.at(match.selectionRange.start.lineIndex, match.selectionRange.start.columnIndex),
		TextPosition.at(match.selectionRange.end.lineIndex, match.selectionRange.end.columnIndex),
	);
}

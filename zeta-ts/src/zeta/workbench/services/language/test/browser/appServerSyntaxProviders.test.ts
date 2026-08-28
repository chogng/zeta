import { strict as assert } from 'node:assert';
import test from 'node:test';
import { TextPosition, TextRange } from '../../../../../editor/common/core/text.js';
import { TextModel } from '../../../../../editor/common/model/textModel.js';
import { SyntaxService } from '../../../../../editor/common/languages/syntax/syntaxService.js';
import { DocumentSymbolService } from '../../../../../editor/contrib/documentSymbols/common/documentSymbols.js';
import { FoldingRangeService } from '../../../../../editor/contrib/folding/common/folding.js';
import { SelectionRangeService } from '../../../../../editor/contrib/smartSelect/common/selectionRanges.js';
import { TestLanguageFeaturesService as LanguageFeaturesService } from '../../../../../editor/test/common/testLanguageFeaturesService.js';
import { AppServerSyntaxProviders, syntaxLanguageForEditorLanguage } from '../../browser/appServerSyntaxProviders.js';

test('App Server syntax registers tokens, diagnostics, symbols, folds, and selection ranges through Editor providers', async () => {
	using model = new TextModel('fn main() {\n  /* hi\n  */\n}\n');
	using languages = new LanguageFeaturesService();
	let analyzeCalls = 0;
	let selectionCalls = 0;
	let workerCalls = 0;
	using providers = new AppServerSyntaxProviders(languages, {
		analyze: async params => {
			analyzeCalls += 1;
			return {
				revision: params.revision,
				hasErrors: true,
				tokens: [
					{ kind: 'variable', range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 7 } } },
					{ kind: 'keyword', range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 2 } } },
					{ kind: 'function', range: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } } },
					{ kind: 'comment', range: { start: { lineIndex: 1, columnIndex: 2 }, end: { lineIndex: 2, columnIndex: 4 } } },
				],
				foldingRanges: [{ range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 3, columnIndex: 1 } } }],
				symbols: [{ name: 'main', kind: 'function', range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 3, columnIndex: 1 } }, selectionRange: { start: { lineIndex: 0, columnIndex: 3 }, end: { lineIndex: 0, columnIndex: 7 } } }],
				diagnostics: [{ kind: 'missing', range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 0, columnIndex: 2 } } }],
			};
		},
		selectionRanges: async params => {
			selectionCalls += 1;
			return { revision: params.revision, ranges: [{ range: { start: { lineIndex: 0, columnIndex: 0 }, end: { lineIndex: 3, columnIndex: 1 } } }] };
		},
	});
	using syntax = new SyntaxService(model, languages.syntaxProvider, {
		workerFactory: () => ({
			run: async request => {
				workerCalls += 1;
				return request.lane === 'tokens' ? { lane: 'tokens' as const, value: { tokens: [] } } : { lane: 'diagnostics' as const, value: { diagnostics: [] } };
			},
			dispose() {},
			[Symbol.dispose]() {},
		}),
	});
	using symbols = new DocumentSymbolService(model, languages.documentSymbolProvider);
	using folding = new FoldingRangeService(model, languages.foldingRangeProvider);
	using selections = new SelectionRangeService(model, languages.selectionRangeProvider);

	await syntax.requestAll('rust');
	const documentSymbols = await symbols.provideDocumentSymbols('rust');
	const foldingRanges = await folding.provideFoldingRanges('rust');
	const structural = await selections.provideSelectionRanges('rust', [TextRange.from(TextPosition.at(0, 3), TextPosition.at(0, 7))]);

	assert.equal(analyzeCalls, 1);
	assert.equal(workerCalls, 0);
	assert.deepEqual(syntax.tokens.result!.value.tokens.map(token => [token.range.start.lineIndex, token.range.start.columnIndex, token.range.end.lineIndex, token.range.end.columnIndex, token.tokenType]), [
		[0, 0, 0, 2, 'keyword'],
		[0, 2, 0, 3, 'variable'],
		[0, 3, 0, 7, 'function'],
		[1, 2, 1, 7, 'comment'],
		[2, 0, 2, 4, 'comment'],
	]);
	assert.ok(syntax.diagnostics.result!.value.diagnostics.some(diagnostic => diagnostic.code === 'syntax-missing' && diagnostic.source === 'zeta-syntax'));
	assert.deepEqual(documentSymbols.map(symbol => [symbol.name, symbol.kind]), [['main', 'function']]);
	assert.deepEqual(foldingRanges, [{ startLineIndex: 0, endLineIndex: 3 }]);
	assert.equal(model.getTextInRange(structural[0]!), 'fn main() {\n  /* hi\n  */\n}');
	assert.equal(selectionCalls, 1);
});

test('App Server syntax maps only supported editor languages', () => {
	assert.equal(syntaxLanguageForEditorLanguage('javascriptreact'), 'javascriptreact');
	assert.equal(syntaxLanguageForEditorLanguage('rust'), 'rust');
	assert.equal(syntaxLanguageForEditorLanguage('typescriptreact'), 'typescriptreact');
	assert.equal(syntaxLanguageForEditorLanguage('markdown'), undefined);
});

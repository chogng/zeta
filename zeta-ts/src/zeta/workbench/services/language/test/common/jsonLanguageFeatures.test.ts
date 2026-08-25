import assert from 'node:assert/strict';
import test from 'node:test';
import { parseJsonc } from '../../../../../base/common/jsonc.js';
import { URI } from '../../../../../base/common/uri.js';
import { TextPosition } from '../../../../../editor/common/core/text.js';
import { LanguageCompletionTriggerKind } from '../../../../../editor/common/languages/completion/languageCompletionProviders.js';
import type { LanguageDiagnostic } from '../../../../../editor/common/languages/languageResults.js';
import { TextModel } from '../../../../../editor/common/model/textModel.js';
import type { LanguageDiagnosticsPublisher } from '../../../../../editor/common/services/languageDiagnosticsService.js';
import { JsonSchemaRegistry } from '../../../../../platform/jsonschemas/common/jsonSchemaRegistry.js';
import { acquireJsonLanguageDiagnostics } from '../../common/jsonLanguageDiagnostics.js';
import { createJsonCompletionProvider, createJsonFormattingProvider, createJsonHoverProvider } from '../../common/jsonLanguageFeatures.js';

const resource = URI.parse('test:/nested.jsonc');
const schema = Object.freeze({
	type: 'object' as const,
	properties: Object.freeze({
		editor: Object.freeze({
			type: 'object' as const,
			properties: Object.freeze({
				enabled: Object.freeze({ type: 'boolean' as const, default: true, title: 'Enabled', description: 'Controls the editor.' }),
			}),
		}),
	}),
});

test('generic JSON language features resolve nested schema completion, hover, and formatting', async () => {
	using registry = associatedRegistry();
	const completion = createJsonCompletionProvider(registry);
	using completionModel = new TextModel('{ "editor": { "en\n}');
	const completionPosition = TextPosition.at(0, completionModel.getLineLength(0));
	const completionResult = await completion.provideCompletions({
		requestId: 1,
		languageId: 'jsonc',
		resource,
		position: completionPosition,
		context: { kind: LanguageCompletionTriggerKind.Invoke },
		snapshot: completionModel.createSnapshot(),
	}, new AbortController().signal);
	assert.deepEqual(completionResult?.items.map(item => item.label), ['enabled']);

	using valueCompletionModel = new TextModel('{ "editor": { "enabled": ');
	const valueCompletionResult = await completion.provideCompletions({
		requestId: 2,
		languageId: 'jsonc',
		resource,
		position: TextPosition.at(0, valueCompletionModel.getLineLength(0)),
		context: { kind: LanguageCompletionTriggerKind.Invoke },
		snapshot: valueCompletionModel.createSnapshot(),
	}, new AbortController().signal);
	assert.deepEqual(valueCompletionResult?.items.map(item => item.label), ['true', 'false']);

	using validModel = new TextModel('{"editor":{"enabled":true,// note\n},}');
	const signal = new AbortController().signal;
	const hover = await createJsonHoverProvider(registry).provideHover({
		model: validModel,
		snapshot: validModel.createSnapshot(),
		languageId: 'jsonc',
		signal,
		resource,
		position: TextPosition.at(0, 13),
	}, signal);
	assert.deepEqual(hover?.contents, ['Enabled', 'Controls the editor.', 'Default: true']);

	const edits = await createJsonFormattingProvider().provideDocumentFormattingEdits!({
		model: validModel,
		snapshot: validModel.createSnapshot(),
		languageId: 'jsonc',
		signal,
		resource,
		options: { tabSize: 2, insertSpaces: true },
	}, signal);
	assert.equal(edits.length, 1);
	assert.match(edits[0]!.text, /\/\/ note/u);
	assert.deepEqual(parseJsonc(edits[0]!.text, 'formatted document'), { editor: { enabled: true } });

	const strictJsonEdits = await createJsonFormattingProvider().provideDocumentFormattingEdits!({
		model: validModel,
		snapshot: validModel.createSnapshot(),
		languageId: 'json',
		signal,
		resource,
		options: { tabSize: 2, insertSpaces: true },
	}, signal);
	assert.deepEqual(strictJsonEdits, []);
});

test('JSON resources publish syntax diagnostics and associated schemas add validation', () => {
	using registry = associatedRegistry();
	using model = new TextModel('{ "editor": { "enabled": "yes" } }');
	let diagnostics: readonly LanguageDiagnostic[] = [];
	const publisher: LanguageDiagnosticsPublisher = {
		update(_revision, next): void {
			diagnostics = next;
		},
		dispose(): void {},
		[Symbol.dispose](): void {},
	};
	using registration = acquireJsonLanguageDiagnostics(resource, 'jsonc', model, () => publisher, registry)!;

	assert.match(diagnostics[0]?.message ?? '', /Expected boolean/u);
	model.reset('{ "editor": { "enabled": true, }, }');
	assert.deepEqual(diagnostics, []);

	using strictModel = new TextModel('{ "enabled": true, // comment\n }');
	let strictDiagnostics: readonly LanguageDiagnostic[] = [];
	using strictRegistration = acquireJsonLanguageDiagnostics(URI.parse('test:/unassociated.json'), 'json', strictModel, () => ({
		update(_revision, next): void {
			strictDiagnostics = next;
		},
		dispose(): void {},
		[Symbol.dispose](): void {},
	}), registry)!;
	assert.match(strictDiagnostics.map(diagnostic => diagnostic.message).join('\n'), /Comments/u);
});

function associatedRegistry(): JsonSchemaRegistry {
	const registry = new JsonSchemaRegistry();
	registry.registerSchema('test://schema/nested', schema);
	registry.registerAssociation(resource, 'test://schema/nested');
	return registry;
}

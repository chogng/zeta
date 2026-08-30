import assert from 'node:assert/strict';
import test from 'node:test';
import { Event } from '../../../base/common/event.js';
import { URI } from '../../../base/common/uri.js';
import { DefaultEndOfLine } from '../../common/model.js';
import { ModelService } from '../../common/services/modelService.js';
import { createPieceTreeTextBuffer } from '../../common/model/textBufferFactory.js';
import { EditSources } from '../../common/textModelEditSource.js';
import { InMemoryConfigurationService } from '../../../platform/configuration/common/inMemoryConfigurationService.js';
import { EditorModelConfiguration } from '../../common/config/editorModelConfiguration.js';
import type { ITextResourcePropertiesService } from '../../common/services/textResourceConfiguration.js';

test('ModelService owns model creation options and applies indentation detection', () => {
	using configuration = new InMemoryConfigurationService();
	using service = new ModelService(configuration, new TestTextResourcePropertiesService(configuration));
	const resource = URI.parse('inmemory://model-service/indentation.txt');
	const options = service.getCreationOptions('plaintext', resource, true);
	assert.deepEqual(options, {
		tabSize: 4,
		indentSize: 'tabSize',
		insertSpaces: true,
		detectIndentation: true,
		trimAutoWhitespace: true,
		defaultEOL: process.platform === 'win32' ? DefaultEndOfLine.CRLF : DefaultEndOfLine.LF,
		isForSimpleWidget: true,
		largeFileOptimizations: true,
		bracketPairColorizationOptions: { enabled: true, independentColorPoolPerBracketType: false },
	});

	using model = service.createModel('root\n  child\n    grandchild', {
		languageId: 'plaintext',
		onDidChange: Event.None,
	}, resource, true);
	assert.equal(model.getOptions().tabSize, 2);
	assert.equal(model.getOptions().indentSize, 2);
	assert.equal(model.getOptions().insertSpaces, true);
	assert.equal(model.isForSimpleWidget, true);
});

test('ModelService consumes ITextBufferFactory values and releases factory ownership', () => {
	using configuration = new InMemoryConfigurationService();
	using service = new ModelService(configuration, new TestTextResourcePropertiesService(configuration));
	let disposalCount = 0;
	const factory = {
		create: (_defaultEOL: DefaultEndOfLine) => ({
			textBuffer: createPieceTreeTextBuffer('factory value'),
			disposable: { dispose: () => { disposalCount++; }, [Symbol.dispose]: () => { disposalCount++; } },
		}),
		getFirstLineText: (lengthLimit: number) => 'factory value'.slice(0, lengthLimit),
	};
	const resource = URI.parse('inmemory://model-service/factory.txt');
	using model = service.createModel(factory, null, resource);
	assert.equal(model.getValue(), 'factory value');
	assert.equal(disposalCount, 1);

	const source = EditSources.reloadFromDisk();
	service.updateModel(model, {
		...factory,
		create: () => ({
			textBuffer: createPieceTreeTextBuffer('\uFEFFupdated\r\nvalue'),
			disposable: { dispose: () => { disposalCount++; }, [Symbol.dispose]: () => { disposalCount++; } },
		}),
	}, source);
	assert.equal(model.getValue(), 'updated\r\nvalue');
	assert.equal(model.getValue(undefined, true), 'updated\r\nvalue');
	assert.equal(model.getEOL(), '\r\n');
	assert.equal(disposalCount, 2);
	let observedSource: unknown;
	using listener = model.onDidChangeContent(event => { observedSource = event.detailedReasons[0]; });
	service.updateModel(model, 'latest value', source);
	assert.equal(observedSource, source);
	assert.equal(model.canUndo(), true);
	model.undo();
	assert.equal(model.getValue(), 'updated\r\nvalue');
});

test('ModelService restores closed file history only for identical content', () => {
	using configuration = new InMemoryConfigurationService();
	using service = new ModelService(configuration, new TestTextResourcePropertiesService(configuration));
	const resource = URI.file('/workspace/history.txt');
	const first = service.createModel('before', null, resource);
	first.applyEdits([{ range: first.getFullModelRange(), text: 'after' }]);
	assert.equal(first.canUndo(), true);
	first.dispose();

	using restored = service.createModel('after', null, resource);
	assert.equal(restored.canUndo(), true);
	restored.undo();
	assert.equal(restored.getValue(), 'before');
	restored.dispose();

	using changed = service.createModel('external change', null, resource);
	assert.equal(changed.canUndo(), false);
});

test('ModelService reapplies model options when platform configuration changes', async () => {
	using configuration = new InMemoryConfigurationService();
	using service = new ModelService(configuration, new TestTextResourcePropertiesService(configuration));
	const model = service.createModel('root\n\tchild', null, URI.parse('inmemory://model-service/configuration.txt'));
	await configuration.updateValue(EditorModelConfiguration.detectIndentation, false);
	await configuration.updateValue(EditorModelConfiguration.tabSize, 2);
	await configuration.updateValue(EditorModelConfiguration.indentSize, 'tabSize');
	await configuration.updateValue(EditorModelConfiguration.insertSpaces, false);
	assert.equal(model.getOptions().tabSize, 2);
	assert.equal(model.getOptions().indentSize, 2);
	assert.equal(model.getOptions().insertSpaces, false);
});

class TestTextResourcePropertiesService implements ITextResourcePropertiesService {
	readonly _serviceBrand: undefined;

	constructor(private readonly configuration: InMemoryConfigurationService) {}

	getEOL(resource: URI, language?: string): string {
		const eol = this.configuration.getValue<string>(EditorModelConfiguration.filesEol, { resource, overrideIdentifier: language });
		return eol === 'auto' ? (process.platform === 'win32' ? '\r\n' : '\n') : eol;
	}
}

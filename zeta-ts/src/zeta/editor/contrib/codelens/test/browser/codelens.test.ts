import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { Emitter } from '../../../../../base/common/event.js';
import { URI } from '../../../../../base/common/uri.js';
import { type IStorageService, type IStorageValueChangeEvent, type IWillSaveStateEvent, StorageScope, StorageTarget, type StorageValue, WillSaveStateReason } from '../../../../../platform/storage/common/storage.js';
import { type TextMeasurer } from '../../../../browser/config/fontMeasurements.js';
import { SemanticMouseTargetFactory, SemanticMouseTargetKind } from '../../../../browser/controller/semanticMouseTarget.js';
import { Position } from '../../../../common/core/position.js';
import { Range } from '../../../../common/core/range.js';
import { LanguageFeatureRegistry } from '../../../../common/languageFeatureRegistry.js';
import { TextModel } from '../../../../common/model/textModel.js';
import { type LanguageCodeLensProvider } from '../../common/languageCodeLenses.js';
import { bindCodeLensCacheStorage, codeLensCache } from '../../browser/codeLensCache.js';
import { LanguageCodeLensModel, getLanguageCodeLensModel, resolveLanguageCodeLensItem } from '../../browser/codelens.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	MouseEvent: browserEnvironment.window.MouseEvent,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { View } = await import('../../../../browser/view.js');
const { EditorCodeLensContribution } = await import('../../browser/codelensController.js');

test('CodeLens model preserves provider ownership, provider rank, and independent failures', async () => {
	using model = new TextModel('first\nsecond', { languageId: 'typescript' });
	const providers = new LanguageFeatureRegistry<LanguageCodeLensProvider>();
	let primaryResolveCount = 0;
	let secondaryResolveCount = 0;
	const primary: LanguageCodeLensProvider = {
		provideCodeLenses: () => [
			lens(1, 4, undefined, 'primary-deferred'),
			lens(0, 2, command('primary.immediate', 'Primary')),
		],
		resolveCodeLens: value => {
			primaryResolveCount += 1;
			return { ...value, command: command('primary.resolved', 'Resolved') };
		},
	};
	const secondary: LanguageCodeLensProvider = {
		provideCodeLenses: () => [lens(1, 0, command('secondary.immediate', 'Secondary'))],
		resolveCodeLens: value => {
			secondaryResolveCount += 1;
			return value;
		},
	};
	const broken: LanguageCodeLensProvider = {
		provideCodeLenses: () => { throw new Error('broken provider'); },
	};
	using primaryRegistration = providers.register('typescript', primary);
	using secondaryRegistration = providers.register('typescript', secondary);
	using brokenRegistration = providers.register('typescript', broken);
	const errors: unknown[] = [];
	const signal = new AbortController().signal;

	const result = await getLanguageCodeLensModel({ model, providers, languageId: 'typescript', signal, onError: error => errors.push(error) });

	assert.deepEqual(result.lenses.map(item => item.symbol.command?.id ?? item.symbol.data), [
		'primary.immediate',
		'secondary.immediate',
		'primary-deferred',
	]);
	assert.equal(result.lenses[2]!.provider, primary);
	assert.equal(errors.length, 1);
	const resolved = await resolveLanguageCodeLensItem({ model, languageId: 'typescript', signal, onError: error => errors.push(error) }, result.lenses[2]!);
	assert.equal(resolved?.command?.id, 'primary.resolved');
	assert.equal(primaryResolveCount, 1);
	assert.equal(secondaryResolveCount, 0);
});

test('CodeLens contribution groups one stable widget per line and refreshes provider changes', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	using model = new TextModel('first\nsecond', { languageId: 'typescript' });
	const providers = new LanguageFeatureRegistry<LanguageCodeLensProvider>();
	using changeEmitter = new Emitter<void>();
	let title = 'Deferred';
	let resolveCount = 0;
	const provider: LanguageCodeLensProvider = {
		onDidChange: changeEmitter.event,
		provideCodeLenses: () => [
			lens(1, 0, command('immediate', 'Immediate')),
			lens(1, 5, undefined, 'deferred'),
		],
		resolveCodeLens: value => {
			resolveCount += 1;
			return { ...value, command: command('deferred', title) };
		},
	};
	using registration = providers.register('typescript', provider);
	using viewport = new View({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 300, height: 60 });
	const executions: string[] = [];
	const contributionErrors: unknown[] = [];
	const executeCommand = ((id: string) => executions.push(id)) as unknown as (id: string, args: readonly unknown[] | undefined) => void;
	using contribution = new EditorCodeLensContribution(viewport, providers, 'typescript', undefined, executeCommand, error => contributionErrors.push(error));

	await contribution.getModel();
	let widget = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-codelens');
	const initialWidget = widget;
	assert.equal(viewport.element.querySelectorAll('.stanza-editor-codelens').length, 1);
	assert.equal(widget.getAttribute('aria-label'), 'CodeLens commands');
	assert.equal(widget.style.top, '20px');
	assert.equal(viewport.getPositionContentCoordinates(new Position((1) + 1, (0) + 1)).top, 34);
	assert.deepEqual([...widget.querySelectorAll('button')].map(button => button.textContent), ['Immediate', 'Deferred']);
	assert.equal(resolveCount, 1);
	const pointerTarget = new SemanticMouseTargetFactory(viewport).create({
		clientX: 0,
		clientY: 0,
		target: widget.querySelector('button'),
	});
	assert.equal(pointerTarget?.kind, SemanticMouseTargetKind.Widget);
	viewport.layout({ width: 320, height: 60 });
	await Promise.resolve();
	assert.equal(resolveCount, 1);
	widget.querySelectorAll('button')[1]!.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true }));
	assert.deepEqual(executions, ['deferred']);
	assert.deepEqual(contributionErrors, []);

	title = 'Updated';
	changeEmitter.fire();
	await Promise.resolve();
	await contribution.getModel();
	widget = requiredElement<HTMLElement>(viewport.element, '.stanza-editor-codelens');
	assert.equal(widget, initialWidget);
	assert.deepEqual([...widget.querySelectorAll('button')].map(button => button.textContent), ['Immediate', 'Updated']);
	assert.equal(resolveCount, 2);

	contribution.dispose();
	assert.equal(viewport.element.querySelector('.stanza-editor-codelens'), null);
	dom.window.close();
});

test('CodeLens model waits for every visible resolve batch in the current request', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	using model = new TextModel(Array.from({ length: 12 }, (_, index) => `line ${index}`).join('\n'), { languageId: 'typescript' });
	const providers = new LanguageFeatureRegistry<LanguageCodeLensProvider>();
	const requests: Array<{ readonly value: ReturnType<typeof lens>; readonly resolve: (value: ReturnType<typeof lens>) => void }> = [];
	using registration = providers.register('typescript', {
		provideCodeLenses: () => [lens(0, 0, undefined, 'first'), lens(10, 0, undefined, 'second')],
		resolveCodeLens: value => new Promise(resolve => requests.push({ value, resolve })),
	});
	using viewport = new View({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 300, height: 20 });
	using contribution = new EditorCodeLensContribution(viewport, providers, 'typescript', undefined, () => undefined);
	await waitFor(() => requests.length === 1);

	viewport.scrollTo({ left: 0, top: 210 });
	await waitFor(() => requests.length === 2);
	const currentModel = contribution.getModel();
	let settled = false;
	void currentModel.then(() => { settled = true; });
	requests[1]!.resolve({ ...requests[1]!.value, command: command('second', 'Second') });
	await Promise.resolve();
	await Promise.resolve();
	assert.equal(settled, false);

	requests[0]!.resolve({ ...requests[0]!.value, command: command('first', 'First') });
	assert.deepEqual((await currentModel).lenses.map(item => item.symbol.command?.id), ['first', 'second']);
	dom.window.close();
});

test('CodeLens cache retains labels without executable commands and rejects a different line count', () => {
	const resource = URI.parse('file:///cached.ts');
	const provider: LanguageCodeLensProvider = {
		provideCodeLenses: () => [],
	};
	const codeLensModel = new LanguageCodeLensModel([Object.freeze({ symbol: lens(1, 0, command('source.run', 'Run')), provider })]);
	try {
		codeLensCache.put(resource, 2, codeLensModel);

		assert.deepEqual(codeLensCache.get(resource, 2)?.lenses.map(item => item.symbol.command), [{ id: '', title: 'Run' }]);
		assert.equal(codeLensCache.get(resource, 3), undefined);
	} finally {
		codeLensCache.delete(resource);
	}
});

test('CodeLens contribution shows cached labels as text until fresh commands arrive', async () => {
	const resource = URI.parse('file:///cached-contribution.ts');
	try {
		const firstDom = new JSDOM('<!doctype html><body><main></main></body>');
		using firstModel = new TextModel('first\nsecond', { languageId: 'typescript' });
		const firstProviders = new LanguageFeatureRegistry<LanguageCodeLensProvider>();
		using firstRegistration = firstProviders.register('typescript', {
			provideCodeLenses: () => [lens(1, 0, command('old.run', 'Cached'))],
		});
		using firstViewport = createViewport(firstDom, firstModel);
		using firstContribution = new EditorCodeLensContribution(firstViewport, firstProviders, 'typescript', resource, () => undefined);
		await firstContribution.getModel();
		firstDom.window.close();

		const secondDom = new JSDOM('<!doctype html><body><main></main></body>');
		using secondModel = new TextModel('first\nsecond', { languageId: 'typescript' });
		const secondProviders = new LanguageFeatureRegistry<LanguageCodeLensProvider>();
		let provideFreshLenses: ((value: readonly ReturnType<typeof lens>[]) => void) | undefined;
		const freshLenses = new Promise<readonly ReturnType<typeof lens>[]>(resolve => { provideFreshLenses = resolve; });
		using secondRegistration = secondProviders.register('typescript', {
			provideCodeLenses: () => freshLenses,
		});
		using secondViewport = createViewport(secondDom, secondModel);
		const executions: string[] = [];
		using secondContribution = new EditorCodeLensContribution(secondViewport, secondProviders, 'typescript', resource, id => { executions.push(id); });

		const cachedCommand = requiredElement<HTMLElement>(secondViewport.element, '.stanza-editor-codelens-command');
		assert.deepEqual({ tagName: cachedCommand.tagName, title: cachedCommand.textContent }, { tagName: 'SPAN', title: 'Cached' });
		cachedCommand.dispatchEvent(new secondDom.window.MouseEvent('click', { bubbles: true }));
		assert.deepEqual(executions, []);

		provideFreshLenses!([lens(1, 0, command('fresh.run', 'Fresh'))]);
		await secondContribution.getModel();
		const freshCommand = requiredElement<HTMLButtonElement>(secondViewport.element, 'button.stanza-editor-codelens-command');
		assert.equal(freshCommand.textContent, 'Fresh');
		freshCommand.dispatchEvent(new secondDom.window.MouseEvent('click', { bubbles: true }));
		assert.deepEqual(executions, ['fresh.run']);
		secondDom.window.close();
	} finally {
		codeLensCache.delete(resource);
	}
});

test('CodeLens cache persists workspace line positions without command data', () => {
	const restoredResource = URI.parse('file:///restored.ts');
	const storedResource = URI.parse('file:///stored.ts');
	const switchedResource = URI.parse('file:///switched.ts');
	const closingResource = URI.parse('file:///closing.ts');
	const storage = new TestStorageService({
		'codelens/cache2': JSON.stringify({
			[restoredResource.toString()]: { lineCount: 3, lines: [3, 3, 7] },
			'invalid resource': { lineCount: 1, lines: [0] },
		}),
	});
	let binding: ReturnType<typeof bindCodeLensCacheStorage> | undefined = bindCodeLensCacheStorage(storage);
	try {
		assert.deepEqual(codeLensCache.get(restoredResource, 3)?.lenses.map(item => ({ lineIndex: item.symbol.range.startLineNumber - 1, command: item.symbol.command })), [
			{ lineIndex: 2, command: undefined },
		]);
		const restoredDom = new JSDOM('<!doctype html><body><main></main></body>');
		using restoredModel = new TextModel('first\nsecond\nthird', { languageId: 'typescript' });
		const restoredProviders = new LanguageFeatureRegistry<LanguageCodeLensProvider>();
		using restoredViewport = new View({
			container: requiredElement<HTMLElement>(restoredDom.window.document, 'main'),
			model: restoredModel,
			lineHeight: 20,
			textMeasurer: new FixedTextMeasurer(),
		});
		restoredViewport.layout({ width: 300, height: 20 });
		using restoredContribution = new EditorCodeLensContribution(restoredViewport, restoredProviders, 'typescript', restoredResource, undefined);
		const restoredWidget = requiredElement<HTMLElement>(restoredViewport.element, '.stanza-editor-codelens');
		assert.deepEqual({
			hidden: restoredWidget.hidden,
			zoneTop: restoredWidget.style.top,
			thirdLineTop: restoredViewport.getPositionContentCoordinates(new Position((2) + 1, (0) + 1)).top,
			contentHeight: restoredViewport.viewportLayout.contentSize.height,
		}, {
			hidden: true,
			zoneTop: '40px',
			thirdLineTop: 54,
			contentHeight: 74,
		});
		restoredDom.window.close();

		const provider: LanguageCodeLensProvider = { provideCodeLenses: () => [] };
		codeLensCache.put(storedResource, 2, new LanguageCodeLensModel([
			Object.freeze({ symbol: lens(1, 0, command('source.run', 'Run')), provider }),
		]));
		storage.fireWillSave(WillSaveStateReason.PERIODIC);
		const serialized = storage.get('codelens/cache2', StorageScope.WORKSPACE, '{}');

		assert.deepEqual(JSON.parse(serialized), {
			[restoredResource.toString()]: { lineCount: 3, lines: [3] },
			[storedResource.toString()]: { lineCount: 2, lines: [2] },
		});
		assert.equal(serialized.includes('source.run') || serialized.includes('Run'), false);
		storage.fireExternalChange('codelens/cache2', JSON.stringify({
			[switchedResource.toString()]: { lineCount: 1, lines: [1] },
		}));
		assert.equal(codeLensCache.get(storedResource, 2), undefined);
		assert.deepEqual(codeLensCache.get(switchedResource, 1)?.lenses.map(item => item.symbol.range.getStartPosition().lineNumber), [1]);

		codeLensCache.put(closingResource, 1, new LanguageCodeLensModel([
			Object.freeze({ symbol: lens(0, 0, command('closing.run', 'Closing')), provider }),
		]));
		binding.dispose();
		binding = undefined;
		assert.deepEqual(JSON.parse(storage.get('codelens/cache2', StorageScope.WORKSPACE, '{}'))[closingResource.toString()], { lineCount: 1, lines: [1] });
	} finally {
		binding?.dispose();
		codeLensCache.delete(restoredResource);
		codeLensCache.delete(storedResource);
		codeLensCache.delete(switchedResource);
		codeLensCache.delete(closingResource);
	}
});

function lens(lineIndex: number, columnIndex: number, value?: ReturnType<typeof command>, data?: unknown) {
	const position = new Position((lineIndex) + 1, (columnIndex) + 1);
	return {
		range: Range.fromPositions(position),
		...(value ? { command: value } : {}),
		...(data !== undefined ? { data } : {}),
	};
}

function command(id: string, title: string) {
	return { id, title } as const;
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
	const element = root.querySelector<T>(selector);
	assert.ok(element);
	return element;
}

function createViewport(dom: JSDOM, model: TextModel): InstanceType<typeof View> {
	const viewport = new View({
		container: requiredElement<HTMLElement>(dom.window.document, 'main'),
		model,
		lineHeight: 20,
		textMeasurer: new FixedTextMeasurer(),
	});
	viewport.layout({ width: 300, height: 60 });
	return viewport;
}

async function waitFor(predicate: () => boolean): Promise<void> {
	for (let attempt = 0; attempt < 20; attempt += 1) {
		if (predicate()) return;
		await Promise.resolve();
	}
	assert.fail('Expected asynchronous CodeLens state was not reached');
}

class FixedTextMeasurer implements TextMeasurer {
	public readonly horizontalPadding = 24;
	public readonly contentLeftPadding = 12;

	public refresh(): boolean {
		return false;
	}

	public measureLineWidth(text: string): number {
		return text.length * 10;
	}
}

class TestStorageService implements IStorageService {
	private readonly changeEmitter = new Emitter<IStorageValueChangeEvent>();
	private readonly willSaveEmitter = new Emitter<IWillSaveStateEvent>();
	private readonly values = new Map<string, string>();
	readonly onDidChangeValue = this.changeEmitter.event;
	readonly onWillSaveState = this.willSaveEmitter.event;

	constructor(values: Readonly<Record<string, string>>) {
		for (const [key, value] of Object.entries(values)) this.values.set(key, value);
	}

	public get(key: string, scope: StorageScope, fallbackValue: string): string;
	public get(key: string, scope: StorageScope): string | undefined;
	public get(key: string, _scope: StorageScope, fallbackValue?: string): string | undefined {
		return this.values.get(key) ?? fallbackValue;
	}

	public getBoolean(key: string, scope: StorageScope, fallbackValue: boolean): boolean;
	public getBoolean(key: string, scope: StorageScope): boolean | undefined;
	public getBoolean(_key: string, _scope: StorageScope, fallbackValue?: boolean): boolean | undefined {
		return fallbackValue;
	}

	public getNumber(key: string, scope: StorageScope, fallbackValue: number): number;
	public getNumber(key: string, scope: StorageScope): number | undefined;
	public getNumber(_key: string, _scope: StorageScope, fallbackValue?: number): number | undefined {
		return fallbackValue;
	}

	public store(key: string, value: StorageValue, scope: StorageScope, target: StorageTarget): void {
		if (value === undefined || value === null) this.values.delete(key);
		else this.values.set(key, String(value));
		this.changeEmitter.fire({ key, scope, target, external: false });
	}

	public remove(key: string, scope: StorageScope): void {
		this.values.delete(key);
		this.changeEmitter.fire({ key, scope, target: undefined, external: false });
	}

	public keys(_scope: StorageScope, _target: StorageTarget): readonly string[] {
		return [...this.values.keys()];
	}

	public isNew(_scope: StorageScope): boolean {
		return false;
	}

	public flush(reason: WillSaveStateReason = WillSaveStateReason.PERIODIC): Promise<void> {
		this.fireWillSave(reason);
		return Promise.resolve();
	}

	public fireWillSave(reason: WillSaveStateReason): void {
		this.willSaveEmitter.fire({ reason });
	}

	public fireExternalChange(key: string, value: string): void {
		this.values.set(key, value);
		this.changeEmitter.fire({ key, scope: StorageScope.WORKSPACE, target: StorageTarget.MACHINE, external: true });
	}
}

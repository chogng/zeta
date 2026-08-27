import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { URI } from '../../../../../base/common/uri.js';
import { type DiffComputationRequest, type IDiffComputationService } from '../../../../../editor/common/diff/diffComputationService.js';
import { type LineDiff } from '../../../../../editor/common/diff/lineDiff.js';
import { MenuService } from '../../../../../platform/actions/common/menuService.js';
import { ContextKeyService } from '../../../../../platform/contextkey/common/contextkey.js';
import { ServiceContainer } from '../../../../../platform/instantiation/common/instantiation.js';
import { EditorPaneVisibility } from '../../../../browser/parts/editor/editorPane.js';
import { CommandService } from '../../../../services/commands/common/commandService.js';
import { TextFileContentSource, type ITextFileService, type ResolvedTextFileContent, type TextFileResolveRequest } from '../../../../services/textfile/common/textFileService.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>');
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
})) {
	Object.defineProperty(globalThis, name, { configurable: true, value });
}

const { BrowserTextModelService } = await import('../../../../../editor/browser/services/browserTextModelService.js');
const { BrowserTextResourceStore } = await import('../../../codeEditor/browser/browserTextResourceStore.js');
const { createMultiDiffEditorInput } = await import('../../browser/multiDiffEditorInput.js');
const { MultiDiffEditorPane } = await import('../../browser/multiDiffEditorPane.js');

test('Stanza multi-diff pane resolves every comparison and releases the complete session', async () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const parent = requiredElement<HTMLElement>(dom.window.document, 'main');
	const resourceStore = new BrowserTextResourceStore(new BootstrapTextFiles());
	using models = new BrowserTextModelService(resourceStore);
	using commands = new CommandService(new ServiceContainer());
	using contexts = new ContextKeyService();
	const menus = new MenuService(commands, contexts);
	const pane = new MultiDiffEditorPane({
		modelService: models,
		createComputationService: () => new PaneTestDiffComputationService(),
		lineHeight: 24,
		showLineNumbers: false,
		fileActions: {
			menuService: menus,
			contextMenuProvider: { showContextMenu() {} },
			contextKeyService: contexts,
		},
	});
	pane.create(parent);
	pane.layout({ width: 640, height: 480 });
	await pane.setInput(createMultiDiffEditorInput(URI.parse('zeta-multi-diff:/test'), [
		{
			label: 'src/first.ts',
			original: { resource: URI.parse('git-change:/first/original'), initialText: 'old', label: 'HEAD' },
			modified: { resource: URI.parse('git-change:/first/modified'), initialText: 'new', label: 'Working Tree' },
		},
		{
			label: 'src/second.ts',
			original: { resource: URI.parse('git-change:/second/original'), initialText: 'before', label: 'HEAD' },
			modified: { resource: URI.parse('git-change:/second/modified'), initialText: 'after', label: 'Working Tree' },
		},
	], 'Review changes'), new AbortController().signal);

	assert.equal(parent.querySelectorAll('.stanza-multi-diff-editor-pane').length, 1);
	assert.equal(parent.querySelectorAll('.stanza-multi-diff-editor-section').length, 2);
	assert.equal(parent.querySelectorAll('.stanza-multi-diff-editor-file-actions > .zeta-toolbar').length, 2);
	assert.equal(parent.querySelectorAll('button button').length, 0);
	assert.equal(parent.querySelector('.stanza-multi-diff-editor')?.getAttribute('aria-label'), 'Review changes, 2 files');
	assert.equal(parent.querySelector('.stanza-multi-diff-editor')?.classList.contains('hide-line-numbers'), true);
	pane.focus();
	assert.equal(dom.window.document.activeElement?.classList.contains('stanza-multi-diff-editor'), true);
	pane.setVisible(EditorPaneVisibility.Hidden);
	assert.equal((parent.firstElementChild as HTMLElement).hidden, true);
	pane.clearInput();
	assert.equal(parent.querySelectorAll('.stanza-multi-diff-editor').length, 0);
	pane.dispose();
	assert.equal(parent.children.length, 0);
	dom.window.close();
});

class BootstrapTextFiles implements ITextFileService {
	readonly onDidChangeFiles = () => ({ dispose() {}, [Symbol.dispose]() {} });

	async resolve(request: TextFileResolveRequest): Promise<ResolvedTextFileContent> {
		return {
			resource: request.resource,
			text: request.bootstrapText ?? '',
			source: request.bootstrapText === undefined ? TextFileContentSource.FileSystem : TextFileContentSource.Bootstrap,
			revision: undefined,
			encoding: "utf8",
		};
	}

	async save(): Promise<{ readonly revision: string | undefined }> {
		return { revision: undefined };
	}
}

class PaneTestDiffComputationService implements IDiffComputationService {
	async compute(_request: DiffComputationRequest, signal: AbortSignal): Promise<LineDiff> {
		signal.throwIfAborted();
		return Object.freeze({ rows: Object.freeze([]), hunks: Object.freeze([]) });
	}

	dispose(): void {}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

function requiredElement<T extends Element>(ownerDocument: Document, selector: string): T {
	const element = ownerDocument.querySelector<T>(selector);
	if (!element) throw new Error(`Missing ${selector}`);
	return element;
}

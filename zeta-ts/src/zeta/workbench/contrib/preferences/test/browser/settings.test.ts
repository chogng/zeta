import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import type { IAction } from '../../../../../base/common/actions.js';
import type { IContextMenuProvider } from '../../../../../base/browser/contextmenu.js';
import type { IClipboardService } from '../../../../../platform/clipboard/common/clipboardService.js';
import type { ILocalizationService } from '../../../../../workbench/services/localization/common/localizationService.js';

const browserEnvironment = new JSDOM('<!doctype html><body></body>', {
	pretendToBeVisual: true,
});
Object.defineProperty(browserEnvironment.window.Element.prototype, 'scrollTo', {
	configurable: true,
	value() {},
});
Object.defineProperty(browserEnvironment.window.Element.prototype, 'scrollIntoView', {
	configurable: true,
	value() {},
});
for (const [name, value] of Object.entries({
	window: browserEnvironment.window,
	document: browserEnvironment.window.document,
	Node: browserEnvironment.window.Node,
	Element: browserEnvironment.window.Element,
	HTMLElement: browserEnvironment.window.HTMLElement,
	Event: browserEnvironment.window.Event,
	MouseEvent: browserEnvironment.window.MouseEvent,
	KeyboardEvent: browserEnvironment.window.KeyboardEvent,
	navigator: browserEnvironment.window.navigator,
})) {
	Object.defineProperty(globalThis, name, {
		configurable: true,
		value,
	});
}

const { h } = await import('../../../../../base/browser/dom.js');
const { noEvent } = await import('../../../../../base/common/event.js');
const { DisposableStore } = await import('../../../../../base/common/lifecycle.js');
const { ConfigurationRegistry } = await import('../../../../../platform/configuration/common/configurationRegistry.js');
const { BrowserContextViewService } = await import('../../../../../platform/contextview/browser/contextViewService.js');
const { darkColorTheme } = await import('../../../../../platform/theme/common/colorTheme.js');
const { AccessibilityConfiguration } = await import('../../../../../platform/accessibility/common/accessibility.js');
const { HoverConfiguration } = await import('../../../../../platform/hover/common/hoverService.js');
const { SashConfiguration } = await import('../../../../../workbench/contrib/sash/common/sash.js');
const { WorkbenchConfiguration } = await import('../../../../../workbench/common/configuration.js');
const { WorkbenchThemesRegistry } = await import('../../../../../workbench/common/theme.js');
const { EditorSelectionConfiguration } = await import('../../../../../workbench/common/editorSelectionConfiguration.js');
const { CodeEditorConfiguration } = await import('../../../../../workbench/contrib/codeEditor/common/editorConfiguration.js');
const { WorkspaceSearchConfiguration } = await import('../../../../../workbench/contrib/search/common/searchConfiguration.js');
const { PreferencesEditor } = await import('../../../../../workbench/contrib/preferences/browser/preferencesEditor.js');
const { createSettingsSections, SettingsLayout, SettingsSections } = await import('../../../../../workbench/contrib/preferences/browser/settingsLayout.js');
const { SettingsTree } = await import('../../../../../workbench/contrib/preferences/browser/settingsTree.js');
const { SettingsTreeModel } = await import('../../../../../workbench/contrib/preferences/browser/settingsTreeModels.js');
const { PreferencesService } = await import('../../../../../workbench/services/preferences/browser/preferencesService.js');
const { DefaultSettings, SettingsEditorModel } = await import('../../../../../workbench/services/preferences/common/preferencesModels.js');
const { WorkbenchConfigurationService } = await import('../../../../../workbench/services/configuration/browser/configurationService.js');

const localizationService: ILocalizationService = {
	onDidChange: noEvent,
	whenReady: Promise.resolve(),
	translate: (_bundle, _key, fallback) => fallback,
};

test('DefaultSettings projects only Configuration Registry metadata', () => {
	const registry = new ConfigurationRegistry();
	const visible = registry.registerConfiguration({
		key: 'editor.test.enabled',
		defaultValue: true,
		parse: value => {
			if (typeof value !== 'boolean') throw new TypeError('Expected a boolean');
			return value;
		},
		setting: {
			valueType: 'boolean',
			title: 'Test setting',
			description: 'A registered test setting.',
		},
	});
	registry.registerConfiguration({
		key: 'internal.test.state',
		defaultValue: 'hidden',
		parse: value => String(value),
	});

	const defaults = new DefaultSettings(registry);
	assert.deepEqual(defaults.all.map(setting => setting.id), ['editor.test.enabled']);
	assert.equal(defaults.get(visible).valueType, 'boolean');
});

test('settingsLayout is the single projection from registered settings to sections', () => {
	const defaults = new DefaultSettings();
	const sections = createSettingsSections(defaults.all);
	const model = new SettingsEditorModel(sections);

	assert.deepEqual(SettingsSections.map(section => section.id), ['general', 'appearance', 'editor']);
	assert.deepEqual(model.sectionIds, ['general', 'appearance', 'editor']);
	assert.equal(findSettingSection(sections, AccessibilityConfiguration.underlineLinks.key), 'general');
	assert.equal(findSettingSection(sections, HoverConfiguration.delay.key), 'general');
	assert.equal(findSettingSection(sections, SashConfiguration.size.key), 'general');
	assert.equal(findSettingSection(sections, WorkbenchConfiguration.colorTheme.key), 'appearance');
	assert.equal(findSettingSection(sections, EditorSelectionConfiguration.defaultNewDocumentEditor.key), 'editor');
	assert.equal(findSettingSection(sections, CodeEditorConfiguration.fontFamily.key), 'editor');
	assert.equal(findSettingSection(sections, WorkspaceSearchConfiguration.maxResults.key), 'editor');
	assert.equal(defaults.all.every(setting => ['boolean', 'number', 'select', 'text'].includes(setting.valueType)), true);
	const themeSetting = defaults.get(WorkbenchConfiguration.colorTheme);
	assert.equal(themeSetting.valueType, 'select');
	if (themeSetting.valueType === 'select') {
		using contributedTheme = WorkbenchThemesRegistry.registerColorTheme({
			...darkColorTheme,
			id: 'test-preferences-dynamic-theme',
			label: 'Dynamic test theme',
		});
		assert.equal(themeSetting.options.some(option => option.value === 'test-preferences-dynamic-theme'), true);
	}

	model.dispose();
});

test('SettingsLayout validates stable configuration identities', () => {
	const fontFamily = new DefaultSettings().get(CodeEditorConfiguration.fontFamily);
	const layout = new SettingsLayout('editor', [{
		id: 'typography',
		title: 'Typography',
		description: 'Editor typography.',
		settings: [fontFamily],
	}]);

	assert.equal(layout.nodes[0]?.element.id, 'editor.group.typography');
	assert.equal(layout.nodes[0]?.children?.[0]?.element.id, CodeEditorConfiguration.fontFamily.key);
	assert.deepEqual(layout.nodes[0]?.children?.[0]?.element.keywords, [CodeEditorConfiguration.fontFamily.key]);
	assert.throws(() => new SettingsLayout('editor', [{
		id: 'typography',
		title: 'Typography',
		description: '',
		settings: [fontFamily, fontFamily],
	}]), /Duplicate Settings setting ID/);
	assert.throws(() => new SettingsLayout('', []), /section ID must not be empty/);
	assert.throws(() => new SettingsLayout('editor', [{
		id: 'typography',
		title: 'Typography',
		description: '',
		settings: [{ ...fontFamily, id: 'editor.font\0Family' }],
	}]), /must not contain control characters/);
});

test('Settings tree preserves item identity while filtering and updating', () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const model = disposables.add(new SettingsTreeModel<string>());
	model.setChildren([{
		element: { kind: 'group', id: 'appearance.colors', title: 'Colors', description: 'Choose the active color scheme.' },
		children: [
			{ element: { kind: 'item', id: 'appearance.colors.theme', title: 'Theme', description: 'Choose a theme.', value: 'Theme' } },
			{ element: { kind: 'item', id: 'appearance.colors.font', title: 'Font family', description: 'Choose a UI font.', value: 'Font' } },
		],
	}]);
	const disposedItems: string[] = [];
	const renderer = disposables.add(new SettingsTree(ownerDocument.body, {
		model,
		rootClassName: 'test-settings-tree',
		groupClassName: 'test-settings-group',
		groupDescriptionClassName: 'test-settings-group-description',
		itemsClassName: 'test-settings-items',
		renderItem: item => {
			const element = h(ownerDocument, 'article');
			element.textContent = item.value;
			return element;
		},
		updateItem: (item, element) => { element.textContent = item.value; },
		disposeItem: item => disposedItems.push(item.id),
	}));

	const themeElement = renderer.getItemElement('appearance.colors.theme');
	assert.ok(themeElement);
	model.setQuery('font family');
	assert.deepEqual(model.visibleItems.map(item => item.id), ['appearance.colors.font']);
	model.setQuery('');
	assert.equal(renderer.getItemElement('appearance.colors.theme'), themeElement);
	model.setNodeChildren('appearance.colors', [{
		element: { kind: 'item', id: 'appearance.colors.theme', title: 'Theme', description: 'Choose a theme.', value: 'Updated Theme' },
	}]);
	assert.equal(themeElement.textContent, 'Updated Theme');
	assert.deepEqual(disposedItems, ['appearance.colors.font']);
	assert.throws(() => model.setChildren([
		{ element: { kind: 'item', id: 'duplicate', title: 'One', description: '', value: 'one' } },
		{ element: { kind: 'item', id: 'duplicate', title: 'Two', description: '', value: 'two' } },
	]), /Duplicate tree node ID/);
});

test('PreferencesEditor renders and updates registry-backed settings only', async () => {
	using disposables = new DisposableStore();
	const ownerDocument = browserEnvironment.window.document;
	ownerDocument.body.replaceChildren();
	const root = h(ownerDocument, 'div');
	const trigger = h(ownerDocument, 'button');
	trigger.textContent = 'Open Settings';
	root.append(trigger);
	ownerDocument.body.append(root);
	trigger.focus();

	const copied: string[] = [];
	let menuActions: readonly IAction[] = [];
	const clipboardService: IClipboardService = {
		writeText: value => {
			copied.push(value);
			return Promise.resolve();
		},
	};
	const contextMenuProvider: IContextMenuProvider = {
		showContextMenu: options => {
			menuActions = options.actions;
		},
	};
	const preferences = disposables.add(new PreferencesService());
	const configuration = disposables.add(new WorkbenchConfigurationService());
	disposables.add(new PreferencesEditor({
		clipboardService,
		configurationService: configuration,
		container: root,
		contextMenuProvider,
		contextViewProvider: disposables.add(new BrowserContextViewService(root)),
		localizationService,
		preferencesService: preferences,
	}));

	preferences.openSettings();
	const host = root.querySelector<HTMLElement>('.zeta-modal-editor-host');
	assert.ok(host);
	assert.equal(host.hidden, false);
	assert.equal(root.querySelector('.zeta-modal-editor')?.getAttribute('role'), 'dialog');
	assert.deepEqual(
		[...root.querySelectorAll<HTMLElement>('[data-settings-section-id]')].map(element => element.dataset.settingsSectionId),
		['general', 'appearance', 'editor'],
	);
	assert.equal(root.querySelector('[data-settings-section-id="models"]'), null);
	assert.ok(root.querySelector(`[data-settings-item-id="${AccessibilityConfiguration.underlineLinks.key}"]`));
	assert.ok(root.querySelector(`[data-settings-item-id="${HoverConfiguration.delay.key}"]`));

	const underline = root.querySelector<HTMLInputElement>(`[data-configuration-key="${AccessibilityConfiguration.underlineLinks.key}"]`);
	assert.ok(underline);
	underline.click();
	await nextTurn();
	assert.equal(configuration.getValue(AccessibilityConfiguration.underlineLinks), true);

	const hoverDelay = root.querySelector<HTMLInputElement>(`[data-configuration-key="${HoverConfiguration.delay.key}"]`);
	assert.ok(hoverDelay);
	hoverDelay.value = '750';
	hoverDelay.dispatchEvent(new browserEnvironment.window.Event('change', { bubbles: true }));
	await nextTurn();
	assert.equal(configuration.getValue(HoverConfiguration.delay), 750);

	root.querySelector<HTMLButtonElement>(`[data-settings-item-id="${HoverConfiguration.delay.key}"] .zeta-setting-item-actions-trigger`)?.click();
	const copyAction = menuActions.find(action => action.id === 'settings.copySettingId');
	assert.ok(copyAction);
	await copyAction.run();
	assert.deepEqual(copied, [HoverConfiguration.delay.key]);

	preferences.openSettings('editor');
	assert.equal(root.querySelector<HTMLElement>('[data-settings-container]')?.dataset.activeSettingsSection, 'editor');
	assert.ok(root.querySelector(`[data-settings-item-id="${CodeEditorConfiguration.fontFamily.key}"]`));
	assert.equal(root.querySelector('[data-settings-item-id^="models.item."]'), null);

	const search = root.querySelector<HTMLInputElement>('.zeta-settings-search input');
	assert.ok(search);
	search.value = 'font family';
	search.dispatchEvent(new browserEnvironment.window.Event('input', { bubbles: true }));
	assert.equal(root.querySelectorAll('.zeta-settings-content-tree [data-settings-item-id]').length, 1);
	assert.ok(root.querySelector(`[data-settings-item-id="${CodeEditorConfiguration.fontFamily.key}"]`));

	preferences.closeSettings();
	assert.equal(host.hidden, true);
	assert.equal(ownerDocument.activeElement, trigger);
});

function findSettingSection(sections: ReturnType<typeof createSettingsSections>, settingId: string): string | undefined {
	return sections.find(section => section.groups.some(group => group.settings.some(setting => setting.id === settingId)))?.sectionId;
}

async function nextTurn(): Promise<void> {
	await new Promise<void>(resolve => setTimeout(resolve, 0));
}

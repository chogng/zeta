import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { Button, type ButtonPresentation } from '../../../../base/browser/ui/button/button.js';
import { DisposableOwner, ResettableDisposableGroup } from '../../../../base/common/lifecycle.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import type { IDialogService } from '../../../../platform/dialogs/common/dialogs.js';
import { ColorId, darkColorTheme, type IColorTheme, lightColorTheme } from '../../../../platform/theme/common/colorTheme.js';
import { isDarkColorScheme } from '../../../../platform/theme/common/theme.js';
import type { IThemeService } from '../../../../platform/theme/common/themeService.js';
import { parseUserColorTheme, serializeUserColorThemeDraft } from '../../../../platform/theme/common/userColorTheme.js';
import { WorkbenchConfiguration } from '../../../common/configuration.js';
import { SystemColorThemePreference, WorkbenchThemesRegistry } from '../../../common/theme.js';
import type { IUserThemeService } from '../../../common/userThemes.js';
import { SettingsItemActions } from './settingsItemActions.js';

interface ThemePreferenceItemOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly dialogService: IDialogService;
	readonly themeService: IThemeService;
	readonly userThemeService: IUserThemeService;
}

interface ThemeOptionDescriptor {
	readonly value: string;
	readonly label: string;
	readonly description: string;
	readonly previewThemes: readonly IColorTheme[];
}

type ThemeDraft =
	| { readonly kind: 'create'; readonly originalTheme: IColorTheme; source: string }
	| { readonly kind: 'update'; readonly originalTheme: IColorTheme; source: string; readonly themeId: string };

/** Owns color-theme selection and user-theme editing independently of the Settings shell. */
export class ThemePreferenceItem extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly renderBindings = this.own(new ResettableDisposableGroup());
	private themeDraft: ThemeDraft | undefined;
	private themeMessage = '';

	constructor(document: Document, private readonly options: ThemePreferenceItemOptions) {
		super();
		this.element = h(document, 'div');
		this.element.className = 'zeta-appearance-settings';
		this.render();
		this.own(options.configurationService.onDidChangeConfiguration(event => {
			if (event.affectsConfiguration(WorkbenchConfiguration.colorTheme)) this.render();
		}));
		this.own(WorkbenchThemesRegistry.onDidChange(() => this.render()));
		this.defer(() => {
			if (this.themeDraft) this.options.themeService.setColorTheme(this.themeDraft.originalTheme);
			this.element.remove();
		});
	}

	public cancelPendingChanges(): void {
		if (!this.themeDraft) return;
		this.options.themeService.setColorTheme(this.themeDraft.originalTheme);
		this.themeDraft = undefined;
		this.themeMessage = '';
		this.render();
	}

	private render(): void {
		this.renderBindings.clear();
		const document = this.element.ownerDocument;
		const item = h(document, 'div');
		item.className = 'zeta-theme-setting-item';
		const group = h(document, 'fieldset');
		group.className = 'zeta-theme-setting';
		const legend = h(document, 'legend');
		legend.textContent = 'Color theme';
		const hint = h(document, 'p');
		hint.className = 'zeta-theme-setting-hint';
		hint.textContent = 'Choose an appearance or keep Zeta synchronized with your operating system.';
		const themeOptionsDomNode = h(document, 'div');
		themeOptionsDomNode.className = 'zeta-theme-options';
		const preference = this.options.configurationService.getValue(WorkbenchConfiguration.colorTheme);
		for (const descriptor of themeOptions(this.options.userThemeService)) {
			const label = h(document, 'label');
			label.className = 'zeta-theme-option';
			label.dataset.themePreference = descriptor.value;
			const input = h(document, 'input');
			input.type = 'radio';
			input.name = 'zeta-color-theme';
			input.value = descriptor.value;
			input.checked = preference === descriptor.value;
			const preview = h(document, 'span');
			preview.className = 'zeta-theme-preview';
			applyThemePreview(preview, descriptor.previewThemes);
			preview.setAttribute('aria-hidden', 'true');
			const copy = h(document, 'span');
			copy.className = 'zeta-theme-option-copy';
			const title = h(document, 'span');
			title.className = 'zeta-theme-option-title';
			title.textContent = descriptor.label;
			const description = h(document, 'span');
			description.className = 'zeta-theme-option-description';
			description.textContent = descriptor.description;
			copy.append(title, description);
			label.append(input, preview, copy);
			this.renderBindings.add(addDisposableListener(input, 'change', () => {
				if (!input.checked) return;
				if (this.themeDraft) {
					this.options.themeService.setColorTheme(this.themeDraft.originalTheme);
					this.themeDraft = undefined;
				}
				this.themeMessage = '';
				group.disabled = true;
				status.textContent = '';
				void this.options.configurationService.updateValue(WorkbenchConfiguration.colorTheme, descriptor.value).catch((error: unknown) => {
					status.textContent = error instanceof Error ? `Unable to save theme: ${error.message}` : 'Unable to save theme.';
					const currentPreference = this.options.configurationService.getValue(WorkbenchConfiguration.colorTheme);
					for (const candidate of themeOptionsDomNode.querySelectorAll<HTMLInputElement>("input[type='radio']")) {
						candidate.checked = candidate.value === currentPreference;
					}
				}).finally(() => {
					group.disabled = false;
				});
			}));
			themeOptionsDomNode.append(label);
		}
		const status = h(document, 'p');
		status.className = 'zeta-theme-setting-status';
		status.setAttribute('role', 'status');
		status.textContent = this.themeMessage;
		if (this.themeMessage) status.classList.add('is-success');
		const customization = h(document, 'div');
		customization.className = 'zeta-theme-customization';
		this.renderBindings.add(new Button(customization, {
			label: this.activeUserThemeId() ? 'Edit user theme JSON' : 'Create from current theme',
			presentation: 'secondary',
			enabled: this.options.userThemeService.available,
			onClick: () => this.startThemeEditing(),
		}));
		group.append(legend, hint, themeOptionsDomNode, status, customization);
		const draft = this.themeDraft;
		if (draft) group.append(this.renderThemeEditor(document, group, status, draft));
		const userThemeStatus = renderUserThemeStatus(document, this.options.userThemeService);
		if (userThemeStatus) group.append(userThemeStatus);
		item.append(group);
		this.renderBindings.add(new SettingsItemActions(item, {
			label: 'Color theme',
			reference: {
				id: WorkbenchConfiguration.colorTheme.key,
				isDefault: () => this.options.configurationService.getValue(WorkbenchConfiguration.colorTheme) === WorkbenchConfiguration.colorTheme.defaultValue,
				reset: async () => {
					if (this.themeDraft) this.options.themeService.setColorTheme(this.themeDraft.originalTheme);
					this.themeDraft = undefined;
					this.themeMessage = '';
					await this.options.configurationService.resetValue(WorkbenchConfiguration.colorTheme);
				},
			},
			contextMenuProvider: this.options.contextMenuProvider,
			clipboardService: this.options.clipboardService,
			onError: error => {
				status.textContent = error instanceof Error ? error.message : 'Unable to run the setting action.';
			},
		}));
		this.element.replaceChildren(item);
	}

	private activeUserThemeId(): string | undefined {
		const preference = this.options.configurationService.getValue(WorkbenchConfiguration.colorTheme);
		return preference === SystemColorThemePreference || !this.options.userThemeService.sourceFor(preference) ? undefined : preference;
	}

	private startThemeEditing(): void {
		const currentTheme = this.options.themeService.getColorTheme();
		const userThemeId = this.activeUserThemeId();
		const existingSource = userThemeId ? this.options.userThemeService.getSource(userThemeId) : undefined;
		if (userThemeId) {
			if (!existingSource) {
				this.themeMessage = `Unable to read the JSON source for '${userThemeId}'.`;
				this.render();
				return;
			}
			this.themeDraft = { kind: 'update', originalTheme: currentTheme, source: existingSource, themeId: userThemeId };
		} else {
			this.themeDraft = {
				kind: 'create',
				originalTheme: currentTheme,
				source: serializeUserColorThemeDraft(currentTheme, this.availableDraftId(currentTheme), `My ${currentTheme.colorScheme === 'light' ? 'Light' : 'Dark'} Theme`),
			};
		}
		this.themeMessage = '';
		this.render();
		this.element.querySelector<HTMLTextAreaElement>('.zeta-theme-json-editor')?.focus();
	}

	private availableDraftId(theme: IColorTheme): string {
		const base = theme.colorScheme === 'light' ? 'my-light-theme' : 'my-dark-theme';
		let candidate = base;
		let suffix = 2;
		while (WorkbenchThemesRegistry.getColorTheme(candidate)) candidate = `${base}-${suffix++}`;
		return candidate;
	}

	private renderThemeEditor(document: Document, group: HTMLFieldSetElement, status: HTMLParagraphElement, draft: ThemeDraft): HTMLElement {
		const editor = h(document, 'section');
		editor.className = 'zeta-theme-json';
		const heading = h(document, 'h4');
		heading.textContent = draft.kind === 'update' ? 'Edit user theme JSON' : 'New theme from current appearance';
		const hint = h(document, 'p');
		hint.textContent = draft.kind === 'update'
			? 'Valid changes preview immediately. Save updates this user theme; change id and label before using Save As.'
			: 'This is a complete copy of the current Light or Dark theme. Change id, label, and colors, then save it as a new theme.';
		const textarea = h(document, 'textarea');
		textarea.className = 'zeta-theme-json-editor';
		textarea.value = draft.source;
		textarea.spellcheck = false;
		textarea.setAttribute('aria-label', 'User theme JSON');
		const actions = h(document, 'div');
		actions.className = 'zeta-theme-json-actions';
		const preview = (): boolean => {
			draft.source = textarea.value;
			try {
				const theme = parseUserColorTheme(draft.source);
				this.options.themeService.setColorTheme(theme);
				status.textContent = `Previewing ${theme.label}.`;
				status.classList.add('is-success');
				return true;
			} catch (error) {
				status.textContent = error instanceof Error ? error.message : 'Theme JSON is invalid.';
				status.classList.remove('is-success');
				return false;
			}
		};
		this.renderBindings.add(addDisposableListener(textarea, 'input', () => preview()));
		if (draft.kind === 'update') {
			this.renderBindings.add(themeAction(actions, 'Save', 'primary', () => {
				if (preview()) void this.saveThemeDraft('save', group, status);
			}));
		}
		this.renderBindings.add(themeAction(actions, 'Save As', draft.kind === 'create' ? 'primary' : 'secondary', () => {
			if (preview()) void this.saveThemeDraft('saveAs', group, status);
		}));
		if (draft.kind === 'update') {
			this.renderBindings.add(themeAction(actions, 'Delete', 'danger', () => {
				void this.deleteThemeDraft(group, status);
			}));
		}
		this.renderBindings.add(themeAction(actions, 'Cancel', 'secondary', () => this.cancelPendingChanges()));
		editor.append(heading, hint, textarea, actions);
		return editor;
	}

	private async saveThemeDraft(operation: 'save' | 'saveAs', group: HTMLFieldSetElement, status: HTMLParagraphElement): Promise<void> {
		const draft = this.themeDraft;
		if (!draft) return;
		group.disabled = true;
		status.classList.remove('is-success');
		status.textContent = operation === 'save' ? 'Saving theme…' : 'Saving new theme…';
		try {
			const result = operation === 'save'
				? await this.options.userThemeService.save(draft.kind === 'update' ? draft.themeId : '', draft.source)
				: await this.options.userThemeService.saveAs(draft.source);
			this.themeDraft = undefined;
			this.options.themeService.setColorTheme(result.theme);
			this.themeMessage = `Saved ${result.theme.label} to ${result.file}.`;
			await this.options.configurationService.updateValue(WorkbenchConfiguration.colorTheme, result.theme.id);
			this.render();
		} catch (error) {
			status.textContent = error instanceof Error ? `Unable to save theme: ${error.message}` : 'Unable to save theme.';
			group.disabled = false;
		}
	}

	private async deleteThemeDraft(group: HTMLFieldSetElement, status: HTMLParagraphElement): Promise<void> {
		const draft = this.themeDraft;
		if (!draft || draft.kind !== 'update') return;
		const theme = WorkbenchThemesRegistry.getColorTheme(draft.themeId);
		if (!theme) {
			status.textContent = `User theme is not loaded: ${draft.themeId}`;
			return;
		}
		group.disabled = true;
		const confirmed = await this.options.dialogService.confirm({
			title: 'Delete user theme?',
			message: `Delete “${theme.label}”?`,
			detail: `This permanently deletes ${this.options.userThemeService.sourceFor(theme.id)?.file ?? 'the theme JSON file'} from the user theme folder.`,
			primaryButton: 'Delete',
			cancelButton: 'Cancel',
		});
		if (!confirmed) {
			group.disabled = false;
			return;
		}
		try {
			const result = await this.options.userThemeService.delete(draft.themeId);
			const fallback = isDarkColorScheme(result.colorScheme) ? darkColorTheme : lightColorTheme;
			this.themeDraft = undefined;
			this.options.themeService.setColorTheme(fallback);
			this.themeMessage = `Deleted ${theme.label} (${result.file}) and switched to ${fallback.label}.`;
			try {
				await this.options.configurationService.updateValue(WorkbenchConfiguration.colorTheme, fallback.id);
			} catch (error) {
				this.themeMessage = error instanceof Error
					? `Deleted ${theme.label}, but could not save the fallback theme: ${error.message}`
					: `Deleted ${theme.label}, but could not save the fallback theme.`;
			}
			this.render();
		} catch (error) {
			status.textContent = error instanceof Error ? `Unable to delete theme: ${error.message}` : 'Unable to delete theme.';
			group.disabled = false;
		}
	}
}

function themeOptions(userThemeService: IUserThemeService): readonly ThemeOptionDescriptor[] {
	return [
		{
			value: SystemColorThemePreference,
			label: 'System',
			description: 'Automatically follow the operating system.',
			previewThemes: [lightColorTheme, darkColorTheme],
		},
		...WorkbenchThemesRegistry.getColorThemes().map(theme => {
			const source = userThemeService.sourceFor(theme.id);
			return {
				value: theme.id,
				label: theme.label,
				description: source ? `User theme · ${source.file}` : `Use ${theme.label} on this device.`,
				previewThemes: [theme],
			};
		}),
	];
}

function renderUserThemeStatus(document: Document, userThemeService: IUserThemeService): HTMLElement | undefined {
	if (!userThemeService.directory && userThemeService.issues.length === 0) return undefined;
	const container = h(document, 'div');
	container.className = 'zeta-user-theme-status';
	if (userThemeService.directory) {
		const directory = h(document, 'p');
		directory.textContent = `User theme folder: ${userThemeService.directory}`;
		container.append(directory);
	}
	if (userThemeService.issues.length > 0) {
		const heading = h(document, 'p');
		heading.textContent = 'Some user themes could not be loaded:';
		const list = h(document, 'ul');
		for (const issue of userThemeService.issues) {
			const item = h(document, 'li');
			item.textContent = `${issue.file}: ${issue.message}`;
			list.append(item);
		}
		container.append(heading, list);
	}
	const restart = h(document, 'p');
	restart.textContent = 'Themes saved here are available immediately. Restart Zeta after external file changes.';
	container.append(restart);
	return container;
}

function themeAction(container: HTMLElement, label: string, presentation: ButtonPresentation, onClick: () => void): Button {
	return new Button(container, { label, presentation, onClick });
}

function applyThemePreview(preview: HTMLElement, themes: readonly IColorTheme[]): void {
	const values = (id: string): readonly string[] => themes.map(theme => requiredThemeColor(theme, id));
	preview.style.setProperty('--theme-preview-editor', previewValue(values(ColorId.editorBackground)));
	preview.style.setProperty('--theme-preview-sidebar', previewValue(values(ColorId.sideBarBackground)));
	preview.style.setProperty('--theme-preview-control', previewValue(values(ColorId.inputBackground)));
}

function requiredThemeColor(theme: IColorTheme, id: string): string {
	const value = theme.getColorCss(id);
	if (!value) throw new Error(`Theme '${theme.id}' does not define preview color '${id}'`);
	return value;
}

function previewValue(values: readonly string[]): string {
	if (values.length === 1) return values[0];
	if (values.length === 2) return `linear-gradient(135deg, ${values[0]} 0 50%, ${values[1]} 50%)`;
	throw new Error('Theme previews support one concrete theme or a light/dark pair');
}

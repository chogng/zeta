import './media/settingsItemActions.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { h } from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import type { IAction } from '../../../../base/common/actions.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import { lxiconsLibrary } from '../../../../base/common/lxiconsLibrary.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { SettingReference } from '../../../services/preferences/common/settingsModel.js';
import { setSettingsItemIdentity } from './settingsItem.js';

export interface SettingsItemActionsOptions {
	readonly label: string;
	readonly reference: SettingReference;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly clipboardService: IClipboardService;
	readonly onError?: (error: unknown) => void;
}

/** Gear menu shared by configurable rows in the Settings editor. */
export class SettingsItemActions extends DisposableOwner {
	public readonly element: HTMLSpanElement;
	private readonly trigger: Button;

	constructor(container: HTMLElement, private readonly options: SettingsItemActionsOptions) {
		super();
		this.element = h(container.ownerDocument, 'span');
		this.element.className = 'zeta-setting-item-actions';
		this.trigger = this.own(new Button(this.element, {
			label: `More actions for ${options.label}`,
			title: `More actions for ${options.label}`,
			icon: lxiconsLibrary.gear,
			onClick: () => this.show(),
		}));
		this.trigger.toggleClassName('zeta-setting-item-actions-trigger', true);
		this.trigger.domNode.setAttribute('aria-label', `More actions for ${options.label}`);
		this.trigger.domNode.setAttribute('aria-haspopup', 'menu');
		this.trigger.domNode.setAttribute('aria-expanded', 'false');
		setSettingsItemIdentity(container, options.reference.id, 'setting');
		container.classList.add('zeta-setting-item');
		container.prepend(this.element);
		this.defer(() => {
			container.classList.remove('zeta-setting-item');
			this.element.remove();
		});
	}

	private show(): void {
		if (this.element.classList.contains('is-open')) return;
		const actions: readonly IAction[] = [
			{
				id: 'settings.resetSetting',
				label: 'Reset Setting',
				tooltip: '',
				enabled: !this.options.reference.isDefault(),
				run: () => this.run(this.options.reference.reset),
			},
			{
				id: 'settings.copySettingId',
				label: 'Copy Setting ID',
				tooltip: '',
				enabled: true,
				run: () => this.run(() => this.options.clipboardService.writeText(this.options.reference.id)),
			},
		];
		this.setOpen(true);
		try {
			this.options.contextMenuProvider.showContextMenu({
				anchor: this.trigger.domNode,
				actions,
				onHide: () => this.setOpen(false),
			});
		} catch (error) {
			this.setOpen(false);
			this.options.onError?.(error);
		}
	}

	private run(operation: () => Promise<void>): void {
		void operation().catch(error => this.options.onError?.(error));
	}

	private setOpen(open: boolean): void {
		this.element.classList.toggle('is-open', open);
		this.trigger.domNode.setAttribute('aria-expanded', String(open));
	}
}

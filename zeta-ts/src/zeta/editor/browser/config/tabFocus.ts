import { Emitter, type Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';

/**
 * Host-scoped state for whether Tab moves browser focus or edits the document.
 *
 * The service owns no DOM nodes and no keybindings. A host may share one
 * instance across its editors; when an editor is constructed without an
 * injected instance, its browser runtime creates an editor-local one.
 */
export class TabFocus extends Disposable {
	private readonly changeEmitter = this._register(new Emitter<boolean>());
	private enabled = false;

	readonly onDidChange: Event<boolean> = this.changeEmitter.event;

	get isEnabled(): boolean {
		return this.enabled;
	}

	setEnabled(enabled: boolean): void {
		if (typeof enabled !== 'boolean') throw new TypeError('Tab focus mode must be boolean');
		if (enabled === this.enabled) return;
		this.enabled = enabled;
		this.changeEmitter.fire(enabled);
	}

	toggle(): boolean {
		this.setEnabled(!this.enabled);
		return this.enabled;
	}
}

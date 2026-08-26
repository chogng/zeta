import { stopEvent } from '../../../base/browser/dom.js';
import { Emitter, type Event } from '../../../base/common/event.js';
import { DisposableOwner } from '../../../base/common/lifecycle.js';
import { type CompositionController } from '../controller/compositionController.js';
import { type EditContext, type EditContextTextUpdate } from '../controller/editContext/editContext.js';
import { type EditorViewTextUpdateEvent, ViewController } from './viewController.js';

/** Owns browser input events and converts them into ViewController commands. */
export class ViewInputController extends DisposableOwner {
	private readonly willBeforeInputEmitter = this.own(new Emitter<InputEvent>());
	private readonly willTextUpdateEmitter = this.own(new Emitter<EditorViewTextUpdateEvent>());
	private readonly willKeydownEmitter = this.own(new Emitter<KeyboardEvent>());

	readonly onWillBeforeInput: Event<InputEvent> = this.willBeforeInputEmitter.event;
	readonly onWillTextUpdate: Event<EditorViewTextUpdateEvent> = this.willTextUpdateEmitter.event;
	readonly onWillKeydown: Event<KeyboardEvent> = this.willKeydownEmitter.event;

	constructor(
		private readonly input: EditContext,
		private readonly viewController: ViewController,
		private readonly compositionController: CompositionController,
	) {
		super();
		try {
			this.own(input.onDidBeforeInput(event => this.handleBeforeInput(event)));
			this.own(input.onDidInput(event => {
				if (!event.isComposing || !this.compositionController.composing) this.input.clear();
			}));
			this.own(input.onDidTextUpdate(event => this.handleTextUpdate(event)));
			this.own(input.onDidKeydown(event => this.handleKeydown(event)));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private handleBeforeInput(event: InputEvent): void {
		if (event.defaultPrevented || (event.isComposing && this.compositionController.composing)) return;
		this.willBeforeInputEmitter.fire(event);
		if (event.defaultPrevented) return;

		switch (event.inputType) {
			case 'insertText':
			case 'insertReplacementText':
				if (!event.data) return;
				stopEvent(event);
				this.input.clear();
				this.viewController.type(event.data, event.inputType);
				return;
			case 'insertLineBreak':
			case 'insertParagraph':
				stopEvent(event);
				this.input.clear();
				this.viewController.enter(event.inputType);
				return;
			case 'deleteContentBackward':
				stopEvent(event);
				this.input.clear();
				this.viewController.deleteBackward(event.inputType);
				return;
			case 'deleteContentForward':
				stopEvent(event);
				this.input.clear();
				this.viewController.deleteForward(event.inputType);
				return;
			case 'deleteWordBackward':
				stopEvent(event);
				this.input.clear();
				this.viewController.deleteWordBackward(event.inputType);
				return;
			case 'deleteWordForward':
				stopEvent(event);
				this.input.clear();
				this.viewController.deleteWordForward(event.inputType);
				return;
			case 'deleteSoftLineBackward':
				stopEvent(event);
				this.input.clear();
				this.viewController.deleteSoftLineBackward(event.inputType);
				return;
			case 'deleteSoftLineForward':
				stopEvent(event);
				this.input.clear();
				this.viewController.deleteSoftLineForward(event.inputType);
				return;
			case 'historyUndo':
				stopEvent(event);
				this.input.clear();
				this.viewController.undo();
				return;
			case 'historyRedo':
				stopEvent(event);
				this.input.clear();
				this.viewController.redo();
				return;
			default:
				return;
		}
	}

	private handleTextUpdate(update: EditContextTextUpdate): void {
		if (this.compositionController.composing) return;
		if (update.updateRangeStart === update.updateRangeEnd && update.text.length === 0) return;
		const event = makeTextUpdateEvent(update);
		this.willTextUpdateEmitter.fire(event);
		if (event.defaultPrevented) return;
		this.viewController.applyTextUpdate(update);
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented) return;
		this.viewController.emitKeyDown(event);
		this.willKeydownEmitter.fire(event);
		if (event.defaultPrevented) return;
		if (!event.isComposing && !event.getModifierState('AltGraph')) {
			if (isUndoKeybinding(event)) {
				stopEvent(event);
				this.input.clear();
				this.viewController.undo();
				return;
			}
			if (isRedoKeybinding(event)) {
				stopEvent(event);
				this.input.clear();
				this.viewController.redo();
				return;
			}
		}
		if (!event.isComposing && event.key === 'Insert' && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
			stopEvent(event);
			this.viewController.toggleOvertype();
			return;
		}
		if (
			event.isComposing ||
			event.key !== 'Tab' ||
			event.shiftKey ||
			event.ctrlKey ||
			event.altKey ||
			event.metaKey
		) return;
		if (this.viewController.hasExpandedSelections) return;
		stopEvent(event);
		// Tab is a keyboard command, not a browser text-input transaction. Keep it
		// out of post-edit consumers such as SuggestController, matching VS Code's
		// command dispatch ordering.
		this.viewController.insertTab();
	}
}

function makeTextUpdateEvent(update: EditContextTextUpdate): EditorViewTextUpdateEvent {
	let defaultPrevented = false;
	return {
		...update,
		get defaultPrevented(): boolean {
			return defaultPrevented;
		},
		preventDefault(): void {
			defaultPrevented = true;
		},
	};
}

function isUndoKeybinding(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>): boolean {
	return hasPrimaryModifier(event) && !event.shiftKey && event.key.toLowerCase() === 'z';
}

function isRedoKeybinding(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>): boolean {
	if (!hasPrimaryModifier(event)) return false;
	const key = event.key.toLowerCase();
	return (key === 'z' && event.shiftKey) || (key === 'y' && !event.shiftKey);
}

function hasPrimaryModifier(event: Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'metaKey'>): boolean {
	return !event.altKey && event.ctrlKey !== event.metaKey;
}

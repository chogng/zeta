import { addDisposableListener } from '../../../../base/browser/dom.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import * as nls from '../../../../nls.js';
import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';
import { EditorAction, EditorContributionInstantiation, registerEditorAction, registerTextEditorCapabilityContribution, type ServicesAccessor, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import { Position } from '../../../common/core/position.js';
import { Selection } from '../../../common/core/selection.js';
import { type IEditorContribution } from '../../../common/editorCommon.js';
import { type EditorHitTarget } from '../../../common/viewModel/pointerHitTest.js';

interface ContextMenuRequest {
	readonly position: Position;
	readonly target: EditorHitTarget | undefined;
	readonly clientX: number;
	readonly clientY: number;
}

/** Owns editor hit testing and delegates menu composition to the host. */
export class ContextMenuController extends Disposable implements IEditorContribution {
	static readonly ID = 'editor.contrib.contextmenu';

	static get(editor: ICodeEditor): ContextMenuController | null {
		return editor.getContribution<ContextMenuController>(ContextMenuController.ID);
	}

	constructor(private readonly context: TextEditorContributionContext) {
		super();
		if (!context.options.onShowContextMenu) return;
		this._register(addDisposableListener<MouseEvent>(context.viewport.element, 'contextmenu', event => this.onContextMenu(event)));
		this._register(addDisposableListener<KeyboardEvent>(context.view.element, 'keydown', event => {
			if (event.defaultPrevented || event.key !== 'F10' || !event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return;
			event.preventDefault();
			event.stopPropagation();
			this.showContextMenu();
		}));
	}

	showContextMenu(request?: ContextMenuRequest): void {
		const show = this.context.options.onShowContextMenu;
		if (!show) return;
		const resolved = request ?? this.keyboardRequest();
		try {
			const result = show(resolved);
			if (result) void result.catch(this.context.onLanguageError);
		} catch (error) {
			this.context.onLanguageError(error);
		}
	}

	private onContextMenu(event: MouseEvent): void {
		event.preventDefault();
		event.stopPropagation();
		const target = this.context.viewport.getNearestTargetAtClientPoint({ clientX: event.clientX, clientY: event.clientY });
		const position = target?.kind === 'text'
			? target.position
			: this.context.model.getPositionAt(this.context.model.getValueLength());
		if (!this.context.selectionController.getSelections().some(selection => selection.containsPosition(position))) {
			this.context.selectionController.setCursorSelections([Selection.fromPositions(position)]);
		}
		this.context.view.focus();
		this.showContextMenu({ position, target, clientX: event.clientX, clientY: event.clientY });
	}

	private keyboardRequest(): ContextMenuRequest {
		const position = this.context.selectionController.getSelections()[0]!.getPosition();
		this.context.viewport.revealPosition(position);
		const content = this.context.viewport.getPositionContentCoordinates(position);
		const bounds = this.context.viewport.element.getBoundingClientRect();
		const scroll = this.context.viewport.currentLayout.scrollPosition;
		return {
			position,
			target: undefined,
			clientX: bounds.left + content.left - scroll.left,
			clientY: bounds.top + content.top - scroll.top + content.height,
		};
	}
}

class ShowContextMenu extends EditorAction {
	constructor() {
		super({ id: 'editor.action.showContextMenu', label: nls.localize2('action.showContextMenu.label', 'Show Editor Context Menu'), precondition: undefined });
	}

	run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		ContextMenuController.get(editor)?.showContextMenu();
	}
}

registerTextEditorCapabilityContribution({
	id: ContextMenuController.ID,
	commands: [{ id: 'editor.action.showContextMenu' }],
	runtime: {
		descriptor: new ServiceConstructionDescriptor(ContextMenuController),
		instantiation: EditorContributionInstantiation.BeforeFirstInteraction,
	},
});
registerEditorAction(ShowContextMenu);

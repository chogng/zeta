import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { registerTextEditorCapabilityContribution } from '../../../browser/editorExtensions.js';
import { type View } from '../../../browser/view.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { CursorMoveCommands } from '../../../common/cursor/cursorMoveCommands.js';

/** Routes the line-expansion command through the text editor input owner. */
class LineSelectionController extends Disposable {
	constructor(input: HTMLElement, private readonly viewport: View, private readonly selections: CursorsController) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError('Line selection dependencies must share one text model');
		this._register(addDisposableListener(input, 'keydown', event => {
			if (event.defaultPrevented || event.isComposing || event.getModifierState('AltGraph') || (!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key.toLowerCase() !== 'l') return;
			stopEvent(event);
			const next = CursorMoveCommands.expandLineSelection(viewport.textModel, selections.selections);
			selections.setSelections(next);
			viewport.revealPosition(next[0]!.getPosition());
		}));
	}
}

registerTextEditorCapabilityContribution({
	id: 'editor.contrib.lineSelection',
	install: context => {
		if (context.kind !== 'text') return;
		context.register(new LineSelectionController(context.view.element, context.viewport, context.viewModel));
	},
});

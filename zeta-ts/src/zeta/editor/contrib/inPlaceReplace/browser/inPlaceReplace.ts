import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { registerTextEditorCapabilityContribution } from '../../../browser/editorExtensions.js';
import { type IVersionedEditorWorkerClient } from '../../../browser/services/editorWorkerService.js';
import { type View } from '../../../browser/view.js';
import { createEditorEditCommand } from '../../../common/commands/editorCommand.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { CursorCollection } from '../../../common/cursor/cursorCollection.js';
import { Range } from '../../../common/core/range.js';

class InPlaceReplaceController extends Disposable {
	constructor(
		input: HTMLElement,
		private readonly viewport: View,
		private readonly selections: CursorsController,
		private readonly editorWorker: IVersionedEditorWorkerClient,
		private readonly wordDefinition: () => RegExp,
		private readonly onError: (error: unknown) => void,
	) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError('In-place replace dependencies must share a text model');
		this._register(addDisposableListener(input, 'keydown', event => {
			if (event.defaultPrevented || event.isComposing || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey) return;
			const direction = event.key === ',' ? -1 : event.key === '.' ? 1 : undefined;
			if (direction === undefined) return;
			stopEvent(event);
			void this.replace(direction).catch(this.onError);
		}, true));
	}

	private async replace(direction: -1 | 1): Promise<boolean> {
		const model = this.viewport.textModel;
		const selectionState = this.selections.selections;
		const selection = selectionState[0]!;
		if (selection.startLineNumber !== selection.endLineNumber) return false;
		const result = await this.editorWorker.navigateValueSet(selection, direction > 0, this.wordDefinition());
		if (!result || !CursorCollection.selectionsEqual(this.selections.selections, selectionState)) return false;
		const command = createEditorEditCommand(model, selectionState, [{ range: result.range, text: result.value }]);
		if (!command) return false;
		this.selections.execute(command);
		this.viewport.revealPosition(Range.lift(result.range).getStartPosition());
		return true;
	}
}

registerTextEditorCapabilityContribution({
	id: 'editor.contrib.inPlaceReplace',
	install: context => {
		if (context.kind !== 'text') return;
		context.register(new InPlaceReplaceController(
			context.view.element,
			context.viewport,
			context.viewModel,
			context.editorWorker,
			() => context.configurations.getLanguageConfiguration(context.languageId).getWordDefinition(),
			context.onLanguageError,
		));
	},
});

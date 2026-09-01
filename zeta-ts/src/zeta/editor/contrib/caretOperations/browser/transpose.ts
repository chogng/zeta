import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { KeyCode, KeyMod } from '../../../../base/common/keyCodes.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { OperatingSystem, operatingSystem } from '../../../../base/common/platform.js';
import * as nls from '../../../../nls.js';
import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { KeybindingWeight } from '../../../../platform/keybinding/common/keybindingsRegistry.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';
import { EditorAction, EditorContributionInstantiation, registerEditorAction, registerTextEditorCapabilityContribution, type ServicesAccessor, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import { EditorCommandHistoryMode, type EditorEditCommand } from '../../../common/commands/editorEditCommand.js';
import { MoveOperations } from '../../../common/cursor/cursorMoveOperations.js';
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { type Selection } from '../../../common/core/selection.js';
import { type TextEdit } from '../../../common/languages.js';
import { type TextModel } from '../../../common/model/textModel.js';

const transposeLettersContributionId = 'editor.contrib.transposeLetters';

interface TransposeOperation {
	readonly selectionIndex: number;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly edit: TextEdit;
}

class TransposeLettersAction extends EditorAction {
	constructor() {
		super({
			id: 'editor.action.transposeLetters',
			label: nls.localize2('transposeLetters.label', 'Transpose Letters'),
			precondition: undefined,
			kbOpts: {
				weight: KeybindingWeight.EditorContrib,
				mac: { primary: KeyMod.WinCtrl | KeyCode.KeyT },
			},
			canTriggerInlineEdits: true,
		});
	}

	public run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		editor.getContribution<TransposeLettersContribution>(transposeLettersContributionId)?.transpose();
	}
}

class TransposeLettersContribution extends Disposable {
	constructor(private readonly context: TextEditorContributionContext) {
		super();
		if (operatingSystem === OperatingSystem.Macintosh) {
			this._register(addDisposableListener(context.view.element, 'keydown', event => this.handleKeydown(event)));
		}
	}

	public transpose(): void {
		const command = createTransposeLettersCommand(this.context.model, this.context.selectionController.getSelections());
		if (!command) return;
		this.context.executeCommand('editor.action.transposeLetters', () => this.context.selectionController.execute(command));
		this.context.viewport.revealPosition(this.context.selectionController.getSelections()[0]!.getPosition());
	}

	private handleKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState('AltGraph')) return;
		if (!event.ctrlKey || event.metaKey || event.shiftKey || event.altKey || event.key.toLowerCase() !== 't') return;
		const command = createTransposeLettersCommand(this.context.model, this.context.selectionController.getSelections());
		if (!command) return;
		stopEvent(event);
		this.context.executeCommand('editor.action.transposeLetters', () => this.context.selectionController.execute(command));
		this.context.viewport.revealPosition(this.context.selectionController.getSelections()[0]!.getPosition());
	}
}

function createTransposeLettersCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand | undefined {
	const candidates = selections.flatMap((selection, selectionIndex) => {
		if (!selection.isEmpty()) return [];
		const cursor = selection.getPosition();
		const lineEndColumn = model.getLineContent(cursor.lineNumber).length + 1;
		const end = cursor.column === lineEndColumn ? cursor : MoveOperations.rightPosition(model, cursor.lineNumber, cursor.column);
		const middle = MoveOperations.leftPosition(model, end);
		const begin = MoveOperations.leftPosition(model, middle);
		if (Position.compare(begin, middle) === 0 || Position.compare(middle, end) === 0) return [];
		const range = Range.fromPositions(begin, end);
		const left = model.getTextInRange(Range.fromPositions(begin, middle));
		const right = model.getTextInRange(Range.fromPositions(middle, end));
		return [Object.freeze({
			selectionIndex,
			startOffset: model.offsetAt(begin),
			endOffset: model.offsetAt(end),
			edit: Object.freeze({ range, text: `${right}${left}` }),
		})];
	});
	const operations = selectOperations(candidates);
	if (operations.length === 0) return undefined;
	const operationBySelection = new Map(operations.map(operation => [operation.selectionIndex, operation]));
	return Object.freeze({
		edits: Object.freeze(operations.map(operation => operation.edit)),
		selectionsAfter: Object.freeze(selections.map((selection, selectionIndex) => {
			const operation = operationBySelection.get(selectionIndex);
			const activeOffset = operation?.endOffset ?? model.offsetAt(selection.getPosition());
			return Object.freeze({
				anchorOffset: operation ? activeOffset : model.offsetAt(selection.getSelectionStart()),
				activeOffset,
			});
		})),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function selectOperations(candidates: readonly TransposeOperation[]): readonly TransposeOperation[] {
	const selected: TransposeOperation[] = [];
	for (const candidate of [...candidates].sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset || left.selectionIndex - right.selectionIndex)) {
		const overlapIndex = selected.findIndex(existing => candidate.startOffset < existing.endOffset && existing.startOffset < candidate.endOffset);
		if (overlapIndex < 0) {
			selected.push(candidate);
			continue;
		}
		if (candidate.selectionIndex === 0 && selected[overlapIndex]!.selectionIndex !== 0) selected[overlapIndex] = candidate;
	}
	return Object.freeze(selected.sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset));
}

registerTextEditorCapabilityContribution({
	id: transposeLettersContributionId,
	runtime: {
		descriptor: new ServiceConstructionDescriptor(TransposeLettersContribution),
		instantiation: EditorContributionInstantiation.Eager,
	},
});

registerEditorAction(TransposeLettersAction);

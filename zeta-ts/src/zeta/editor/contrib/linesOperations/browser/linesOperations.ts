import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
import { Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextEdit } from '../../../common/languages.js';
import * as nls from '../../../../nls.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';
import { EditorAction, registerEditorAction, type ServicesAccessor } from '../../../browser/editorExtensions.js';
import { MoveOperations } from '../../../common/cursor/cursorMoveOperations.js';
import { type ICommand, type IEditorContribution } from '../../../common/editorCommon.js';
import { type IActionOptions } from '../../../browser/editorExtensions.js';
import { CopyLinesCommand } from './copyLinesCommand.js';
import { MoveLinesCommand } from './moveLinesCommand.js';
import { SortLinesCommand } from './sortLinesCommand.js';
import { ReplaceCommandThatSelectsText } from '../../../common/commands/replaceCommand.js';
import { ShiftCommand } from '../../../common/commands/shiftCommand.js';
import { EditorAutoIndentStrategy } from '../../../common/config/editorOptions.js';
import { ILanguageConfigurationService } from '../../../common/languages/languageConfigurationRegistry.js';
import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { operatingSystem, OperatingSystem } from '../../../../base/common/platform.js';
import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { EditorContributionInstantiation, registerTextEditorCapabilityContribution, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';

const linesOperationsContributionId = 'editor.contrib.linesOperations';

export enum EditorLineDuplicateDirection {
	Up = "up",
	Down = "down",
}

export enum EditorLineMoveDirection {
	Up = "up",
	Down = "down",
}

/** Selects whether a blank line is inserted before or after selected line groups. */
export enum EditorLineInsertDirection {
	Before = "before",
	After = "after",
}

interface OffsetEdit {
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly edit: TextEdit;
}

interface TransposeOperation {
	readonly selectionIndex: number;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly edit: TextEdit;
}

class LinesOperationsContribution extends Disposable implements IEditorContribution {
	constructor(private readonly context: TextEditorContributionContext) {
		super();
		this._register(addDisposableListener(context.view.element, 'keydown', event => this.onKeydown(event)));
	}

	transpose(): void {
		const command = createTransposeCommand(this.context.model, this.context.selectionController.selections);
		if (!command) return;
		this.run('editor.action.transpose', command);
	}

	private onKeydown(event: KeyboardEvent): void {
		if (event.defaultPrevented || event.isComposing || event.getModifierState('AltGraph')) return;
		if (event.key === 'Tab' && !event.ctrlKey && !event.altKey && !event.metaKey) {
			const hasRange = this.context.selectionController.selections.some(selection => !selection.isEmpty());
			if (!event.shiftKey && !hasRange) return;
			stopEvent(event);
			const unshift = event.shiftKey;
			const options = this.context.model.getOptions();
			this.runCommands(
				unshift ? 'editor.action.outdentLines' : 'editor.action.indentLines',
				this.context.selectionController.selections.map(selection => new ShiftCommand(selection, {
					isUnshift: unshift,
					tabSize: options.tabSize,
					indentSize: options.indentSize,
					insertSpaces: options.insertSpaces,
					useTabStops: true,
					autoIndent: EditorAutoIndentStrategy.None,
				}, this.context.configurations)),
			);
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === 'k') {
			stopEvent(event);
			this.run('editor.action.deleteLines', createDeleteLinesCommand(this.context.model, this.context.selectionController.selections));
			return;
		}
		if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key === 'Enter') {
			stopEvent(event);
			const direction = event.shiftKey ? EditorLineInsertDirection.Before : EditorLineInsertDirection.After;
			this.run(
				direction === EditorLineInsertDirection.Before ? 'editor.action.insertLineBefore' : 'editor.action.insertLineAfter',
				createInsertLineCommand(this.context.model, this.context.selectionController.selections, direction),
			);
			return;
		}
		if (isJoinChord(event)) {
			stopEvent(event);
			this.run('editor.action.joinLines', createJoinLinesCommand(this.context.model, this.context.selectionController.selections));
			return;
		}
		if (!event.altKey || event.ctrlKey || event.metaKey) return;
		const direction = event.key === 'ArrowUp' ? 'up' : event.key === 'ArrowDown' ? 'down' : undefined;
		if (!direction) return;
		if (!event.shiftKey) {
			stopEvent(event);
			this.runCommands(
				direction === 'up' ? 'editor.action.moveLinesUpAction' : 'editor.action.moveLinesDownAction',
				this.context.selectionController.selections.map(selection => new MoveLinesCommand(
					selection,
					direction === 'down',
					EditorAutoIndentStrategy.None,
					this.context.configurations,
				)),
			);
			return;
		}
		if (operatingSystem === OperatingSystem.Linux) return;
		stopEvent(event);
		this.runCommands(
			direction === 'up' ? 'editor.action.copyLinesUpAction' : 'editor.action.copyLinesDownAction',
			this.context.selectionController.selections.map(selection => new CopyLinesCommand(selection, direction === 'down')),
		);
	}

	private run(id: string, command: EditorEditCommand): void {
		this.context.executeCommand(id, () => this.context.selectionController.execute(command));
		this.context.viewport.revealPosition(this.context.selectionController.selections[0]!.getPosition());
	}

	private runCommands(id: string, commands: readonly ICommand[]): void {
		this.context.executeCommand(id, () => this.context.selectionController.executeCommands(commands, id));
		this.context.viewport.revealPosition(this.context.selectionController.selections[0]!.getPosition());
	}
}

function isJoinChord(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>): boolean {
	if (event.shiftKey || event.altKey || event.key.toLowerCase() !== 'j') return false;
	return operatingSystem === OperatingSystem.Macintosh
		? event.metaKey && !event.ctrlKey
		: event.ctrlKey && !event.metaKey;
}

abstract class CopyLinesAction extends EditorAction {
	constructor(private readonly down: boolean, options: IActionOptions) {
		super(options);
	}

	public run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		const selections = editor.getSelections() ?? [];
		editor.executeCommands(this.id, selections.map(selection => new CopyLinesCommand(selection, this.down)));
	}
}

class CopyLinesUpAction extends CopyLinesAction {
	constructor() {
		super(false, { id: 'editor.action.copyLinesUpAction', label: nls.localize2('lines.copyUp', 'Copy Line Up'), precondition: undefined, canTriggerInlineEdits: true });
	}
}

class CopyLinesDownAction extends CopyLinesAction {
	constructor() {
		super(true, { id: 'editor.action.copyLinesDownAction', label: nls.localize2('lines.copyDown', 'Copy Line Down'), precondition: undefined, canTriggerInlineEdits: true });
	}
}

/** Duplicates selected text, or the selected physical line for an empty selection. */
export class DuplicateSelectionAction extends EditorAction {
	constructor() {
		super({ id: 'editor.action.duplicateSelection', label: nls.localize2('duplicateSelection', 'Duplicate Selection'), precondition: undefined, canTriggerInlineEdits: true });
	}

	public run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		const model = editor.getModel();
		if (!model) return;
		const commands = (editor.getSelections() ?? []).map(selection => selection.isEmpty()
			? new CopyLinesCommand(selection, true)
			: new ReplaceCommandThatSelectsText(
				Selection.fromPositions(selection.getEndPosition()),
				model.getValueInRange(selection),
			));
		editor.executeCommands(this.id, commands);
	}
}

abstract class MoveLinesAction extends EditorAction {
	constructor(private readonly down: boolean, options: IActionOptions) {
		super(options);
	}

	public run(accessor: ServicesAccessor, editor: ICodeEditor): void {
		const configurations = accessor.get(ILanguageConfigurationService);
		const commands = (editor.getSelections() ?? []).map(selection => new MoveLinesCommand(
			selection,
			this.down,
			EditorAutoIndentStrategy.None,
			configurations,
		));
		editor.executeCommands(this.id, commands);
	}
}

class MoveLinesUpAction extends MoveLinesAction {
	constructor() {
		super(false, { id: 'editor.action.moveLinesUpAction', label: nls.localize2('lines.moveUp', 'Move Line Up'), precondition: undefined, canTriggerInlineEdits: true });
	}
}

class MoveLinesDownAction extends MoveLinesAction {
	constructor() {
		super(true, { id: 'editor.action.moveLinesDownAction', label: nls.localize2('lines.moveDown', 'Move Line Down'), precondition: undefined, canTriggerInlineEdits: true });
	}
}

export abstract class AbstractSortLinesAction extends EditorAction {
	constructor(private readonly descending: boolean, options: IActionOptions) {
		super(options);
	}

	public run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		const model = editor.getModel();
		if (!model) return;
		let selections = editor.getSelections() ?? [];
		if (selections.length === 1 && selections[0]!.isSingleLine()) {
			selections = [new Selection(1, 1, model.getLineCount(), model.getLineMaxColumn(model.getLineCount()))];
		}
		if (!selections.every(selection => SortLinesCommand.canRun(model, selection, this.descending))) return;
		editor.executeCommands(this.id, selections.map(selection => new SortLinesCommand(selection, this.descending)));
	}
}

export class SortLinesAscendingAction extends AbstractSortLinesAction {
	constructor() {
		super(false, { id: 'editor.action.sortLinesAscending', label: nls.localize2('lines.sortAscending', 'Sort Lines Ascending'), precondition: undefined, canTriggerInlineEdits: true });
	}
}

export class SortLinesDescendingAction extends AbstractSortLinesAction {
	constructor() {
		super(true, { id: 'editor.action.sortLinesDescending', label: nls.localize2('lines.sortDescending', 'Sort Lines Descending'), precondition: undefined, canTriggerInlineEdits: true });
	}
}

export class TransposeAction extends EditorAction {
	constructor() {
		super({
			id: 'editor.action.transpose',
			label: nls.localize2('editor.transpose', 'Transpose Characters around the Cursor'),
			precondition: undefined,
			canTriggerInlineEdits: true,
		});
	}

	public run(_accessor: ServicesAccessor, editor: ICodeEditor): void {
		editor.getContribution<LinesOperationsContribution>(linesOperationsContributionId)?.transpose();
	}
}

/** Creates the line-operations transpose transaction for all collapsed selections. */
export function createTransposeCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand | undefined {
	const candidates = selections.flatMap((selection, selectionIndex) => {
		if (!selection.isEmpty()) return [];
		const cursor = selection.getPosition();
		const isLineEnd = cursor.column === model.getLineContent(cursor.lineNumber).length + 1;
		if (isLineEnd && cursor.lineNumber === model.lineCount) return [];
		const begin = cursor.column === 1 ? cursor : MoveOperations.leftPosition(model, cursor);
		const end = MoveOperations.rightPosition(model, cursor.lineNumber, cursor.column);
		if (Position.compare(cursor, end) === 0) return [];
		const range = Range.fromPositions(begin, end);
		const left = model.getTextInRange(Range.fromPositions(begin, cursor));
		const right = model.getTextInRange(Range.fromPositions(cursor, end));
		return [Object.freeze({
			selectionIndex,
			startOffset: model.offsetAt(begin),
			endOffset: model.offsetAt(end),
			edit: Object.freeze({ range, text: `${right}${left}` }),
		})];
	});
	const operations = selectTransposeOperations(candidates);
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

function selectTransposeOperations(candidates: readonly TransposeOperation[]): readonly TransposeOperation[] {
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

/** Deletes the union of physical lines selected by every cursor. */
export function createDeleteLinesCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const edits = groups.flatMap<OffsetEdit>(group => deleteLineGroup(model, group));
	return createLineOperationCommand(model, selections, edits);
}

/** Duplicates the union of physical lines selected by every cursor. */
export function createDuplicateLinesCommand(model: TextModel, selections: readonly Selection[], direction: EditorLineDuplicateDirection): EditorEditCommand {
	if (!Object.values(EditorLineDuplicateDirection).includes(direction)) {
		throw new TypeError("Unknown editor line duplicate direction");
	}
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const edits = groups.flatMap<OffsetEdit>(group => duplicateLineGroup(model, group, direction));
	return createLineOperationCommand(model, selections, edits);
}

/** Moves the union of selected physical lines by one neighboring line. */
export function createMoveLinesCommand(model: TextModel, selections: readonly Selection[], direction: EditorLineMoveDirection): EditorEditCommand {
	if (!Object.values(EditorLineMoveDirection).includes(direction)) {
		throw new TypeError("Unknown editor line move direction");
	}
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const movableGroups = groups.filter(group => direction === EditorLineMoveDirection.Up
		? group.startLineIndex > 0
		: group.endLineIndex + 1 < model.lineCount);
	const edits = movableGroups.map(group => moveLineGroup(model, group, direction));
	const finalText = applyOffsetEdits(model.createVersionedSnapshot().getText(), edits);
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selections.map(selection => Object.freeze({
			anchorOffset: offsetInText(finalText, movePosition(selection.getSelectionStart(), movableGroups, direction)),
			activeOffset: offsetInText(finalText, movePosition(selection.getPosition(), movableGroups, direction)),
		}))),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

/** Inserts one blank line adjacent to each contiguous selected physical-line group. */
export function createInsertLineCommand(model: TextModel, selections: readonly Selection[], direction: EditorLineInsertDirection): EditorEditCommand {
	if (!Object.values(EditorLineInsertDirection).includes(direction)) {
		throw new TypeError("Unknown editor line insertion direction");
	}
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const edits = groups.map(group => insertLineAtGroup(model, group, direction));
	const finalText = applyOffsetEdits(model.createVersionedSnapshot().getText(), edits);
	const nextSelections = groups.map((group, index) => Selection.fromPositions(new Position((insertedLineIndex(group, index, direction)) + 1, (0) + 1)));
	const primaryIndex = primaryInsertedGroupIndex(selections, groups);
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(nextSelections.map(selection => Object.freeze({
			anchorOffset: offsetInText(finalText, selection.getSelectionStart()),
			activeOffset: offsetInText(finalText, selection.getPosition()),
		}))),
		primarySelectionIndex: primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

interface JoinTarget {
	readonly start: Position;
	readonly end: Position;
	readonly primary: boolean;
}

interface JoinEdit {
	readonly target: JoinTarget;
	readonly range: Range;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly selectionStart: number;
	readonly selectionEnd: number;
}

/** Joins each non-overlapping cursor or range group as one editor transaction. */
export function createJoinLinesCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
	const edits = reduceJoinTargets(selections).map(target => joinEdit(model, target));
	if (edits.every(edit => edit.startOffset === edit.endOffset)) return unchangedLineCommand(model, selections);
	const operations: TextEdit[] = [];
	const selectionsAfter: TextSelectionOffsets[] = [];
	let primarySelectionIndex = 0;
	let delta = 0;
	for (const edit of edits) {
		const base = edit.startOffset + delta;
		selectionsAfter.push({ anchorOffset: base + edit.selectionStart, activeOffset: base + edit.selectionEnd });
		if (edit.target.primary) primarySelectionIndex = selectionsAfter.length - 1;
		if (edit.startOffset === edit.endOffset) continue;
		operations.push({ range: edit.range, text: edit.text });
		delta += edit.text.length - (edit.endOffset - edit.startOffset);
	}
	return {
		edits: Object.freeze(operations),
		selectionsAfter: Object.freeze(selectionsAfter),
		primarySelectionIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	};
}

function reduceJoinTargets(selections: readonly Selection[]): readonly JoinTarget[] {
	const ordered = selections.map((selection, index) => ({
		start: selection.getStartPosition(),
		end: selection.getEndPosition(),
		primary: index === 0,
	})).sort((left, right) => Position.compare(left.start, right.start) || Position.compare(left.end, right.end));
	const result: JoinTarget[] = [];
	for (const target of ordered) {
		const previous = result.at(-1);
		if (!previous) {
			result.push(target);
			continue;
		}
		const previousCollapsed = Position.compare(previous.start, previous.end) === 0;
		if (previousCollapsed && previous.end.lineNumber === target.start.lineNumber) {
			result[result.length - 1] = { ...target, primary: previous.primary || target.primary };
			continue;
		}
		const separate = previousCollapsed
			? target.start.lineNumber > previous.end.lineNumber + 1
			: target.start.lineNumber > previous.end.lineNumber;
		if (separate) {
			result.push(target);
			continue;
		}
		result[result.length - 1] = { start: previous.start, end: target.end, primary: previous.primary || target.primary };
	}
	return result;
}

function joinEdit(model: TextModel, target: JoinTarget): JoinEdit {
	const collapsed = Position.compare(target.start, target.end) === 0;
	const endLineNumber = collapsed ? Math.min(target.start.lineNumber + 1, model.lineCount) : target.end.lineNumber;
	if (endLineNumber === target.start.lineNumber) {
		const lineStart = new Position(target.start.lineNumber, 1);
		const offset = model.offsetAt(lineStart);
		return {
			target,
			range: Range.fromPositions(lineStart),
			startOffset: offset,
			endOffset: offset,
			text: '',
			selectionStart: target.start.column - 1,
			selectionEnd: target.end.column - 1,
		};
	}
	const end = new Position(endLineNumber, model.getLineMaxColumn(endLineNumber));
	const joined = joinText(model, target.start.lineNumber, endLineNumber);
	const selectionTail = model.getLineContent(target.end.lineNumber).length - (target.end.column - 1);
	const boundary = collapsed ? joined.text.length - joined.lastPartLength : target.start.column - 1;
	return {
		target: { ...target, end },
		range: Range.fromPositions(new Position(target.start.lineNumber, 1), end),
		startOffset: model.offsetAt(new Position(target.start.lineNumber, 1)),
		endOffset: model.offsetAt(end),
		text: joined.text,
		selectionStart: boundary,
		selectionEnd: collapsed ? boundary : joined.text.length - selectionTail,
	};
}

function joinText(model: TextModel, startLineNumber: number, endLineNumber: number): { readonly text: string; readonly lastPartLength: number } {
	let text = model.getLineContent(startLineNumber);
	let lastPartLength = 0;
	for (let lineNumber = startLineNumber + 1; lineNumber <= endLineNumber; lineNumber += 1) {
		const part = model.getLineContent(lineNumber).replace(/^[\s\uFEFF\xA0]+/u, '');
		if (!part) {
			lastPartLength = 0;
			continue;
		}
		let separator = text.length > 0 ? ' ' : '';
		if (separator && /[\s\uFEFF\xA0]$/u.test(text)) {
			text = text.replace(/[\s\uFEFF\xA0]+$/u, ' ');
			separator = '';
		}
		text += separator + part;
		lastPartLength = separator.length + part.length;
	}
	return { text, lastPartLength };
}

function unchangedLineCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
	return {
		edits: Object.freeze([]),
		selectionsAfter: Object.freeze(selections.map(selection => ({
			anchorOffset: model.offsetAt(selection.getSelectionStart()),
			activeOffset: model.offsetAt(selection.getPosition()),
		}))),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	};
}

function deleteLineGroup(model: TextModel, group: EditorLineGroup): readonly OffsetEdit[] {
	const first = group.startLineIndex;
	const last = group.endLineIndex;
	if (first === 0 && last === model.lineCount - 1) {
		const start = new Position((0) + 1, (0) + 1);
		const end = new Position((last) + 1, (model.getLineContent((last) + 1).length) + 1);
		return [offsetEdit(model, start, end, "")];
	}
	if (last + 1 < model.lineCount) {
		return [offsetEdit(model, new Position((first) + 1, (0) + 1), new Position((last + 1) + 1, (0) + 1), "")];
	}
	const previousLineIndex = first - 1;
	return [offsetEdit(
		model,
		new Position((previousLineIndex) + 1, (model.getLineContent((previousLineIndex) + 1).length) + 1),
		new Position((last) + 1, (model.getLineContent((last) + 1).length) + 1),
		"",
	)];
}

function duplicateLineGroup(model: TextModel, group: EditorLineGroup, direction: EditorLineDuplicateDirection): readonly OffsetEdit[] {
	const text = Array.from(
		{ length: group.endLineIndex - group.startLineIndex + 1 },
		(_, index) => model.getLineContent((group.startLineIndex + index) + 1),
	).join("\n");
	if (direction === EditorLineDuplicateDirection.Up) {
		return [offsetEdit(model, new Position((group.startLineIndex) + 1, (0) + 1), new Position((group.startLineIndex) + 1, (0) + 1), `${text}\n`)];
	}
	if (group.endLineIndex + 1 < model.lineCount) {
		return [offsetEdit(model, new Position((group.endLineIndex + 1) + 1, (0) + 1), new Position((group.endLineIndex + 1) + 1, (0) + 1), `${text}\n`)];
	}
	const end = new Position((group.endLineIndex) + 1, (model.getLineContent((group.endLineIndex) + 1).length) + 1);
	return [offsetEdit(model, end, end, `\n${text}`)];
}

function moveLineGroup(model: TextModel, group: EditorLineGroup, direction: EditorLineMoveDirection): OffsetEdit {
	if (direction === EditorLineMoveDirection.Up) {
		const previousLineIndex = group.startLineIndex - 1;
		const start = new Position((previousLineIndex) + 1, (0) + 1);
		const end = new Position((group.endLineIndex) + 1, (model.getLineContent((group.endLineIndex) + 1).length) + 1);
		const previous = model.getLineContent((previousLineIndex) + 1);
		const selected = lineGroupText(model, group);
		return offsetEdit(model, start, end, `${selected}\n${previous}`);
	}
	const nextLineIndex = group.endLineIndex + 1;
	const start = new Position((group.startLineIndex) + 1, (0) + 1);
	const end = new Position((nextLineIndex) + 1, (model.getLineContent((nextLineIndex) + 1).length) + 1);
	const selected = lineGroupText(model, group);
	const next = model.getLineContent((nextLineIndex) + 1);
	return offsetEdit(model, start, end, `${next}\n${selected}`);
}

function insertLineAtGroup(model: TextModel, group: EditorLineGroup, direction: EditorLineInsertDirection): OffsetEdit {
	if (direction === EditorLineInsertDirection.Before) {
		const position = new Position((group.startLineIndex) + 1, (0) + 1);
		return offsetEdit(model, position, position, "\n");
	}
	const lineIndex = group.endLineIndex + 1;
	const position = lineIndex < model.lineCount
		? new Position((lineIndex) + 1, (0) + 1)
		: new Position((group.endLineIndex) + 1, (model.getLineContent((group.endLineIndex) + 1).length) + 1);
	return offsetEdit(model, position, position, "\n");
}

function insertedLineIndex(group: EditorLineGroup, precedingInsertions: number, direction: EditorLineInsertDirection): number {
	return direction === EditorLineInsertDirection.Before
		? group.startLineIndex + precedingInsertions
		: group.endLineIndex + precedingInsertions + 1;
}

function primaryInsertedGroupIndex(selections: readonly Selection[], groups: readonly EditorLineGroup[]): number {
	const primaryLines = selectedLineIndices([selections[0]!]);
	for (const lineIndex of primaryLines) {
		const groupIndex = groups.findIndex(group =>
			lineIndex >= group.startLineIndex && lineIndex <= group.endLineIndex
		);
		if (groupIndex >= 0) return groupIndex;
	}
	return 0;
}

function lineGroupText(model: TextModel, group: EditorLineGroup): string {
	return Array.from(
		{ length: group.endLineIndex - group.startLineIndex + 1 },
		(_, index) => model.getLineContent((group.startLineIndex + index) + 1),
	).join("\n");
}

function createLineOperationCommand(model: TextModel, selections: readonly Selection[], edits: readonly OffsetEdit[]): EditorEditCommand {
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selections.map(selection => Object.freeze({
			anchorOffset: mapOffsetThroughEdits(model.offsetAt(selection.getSelectionStart()), edits),
			activeOffset: mapOffsetThroughEdits(model.offsetAt(selection.getPosition()), edits),
		}))),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function offsetEdit(model: TextModel, start: Position, end: Position, text: string): OffsetEdit {
	return Object.freeze({
		startOffset: model.offsetAt(start),
		endOffset: model.offsetAt(end),
		text,
		edit: Object.freeze({ range: Range.fromPositions(start, end), text }),
	});
}

interface EditorLineGroup {
	readonly startLineIndex: number;
	readonly endLineIndex: number;
}

function selectedLineIndices(selections: readonly Selection[]): readonly number[] {
	const indices = new Set<number>();
	for (const selection of selections) {
		const range = selection;
		let endLineIndex = range.endLineNumber - 1;
		if (!selection.isEmpty() && range.endColumn === 1 && endLineIndex > range.startLineNumber - 1) {
			endLineIndex -= 1;
		}
		for (let lineIndex = range.startLineNumber - 1; lineIndex <= endLineIndex; lineIndex += 1) indices.add(lineIndex);
	}
	return Object.freeze([...indices].sort((left, right) => left - right));
}

function contiguousLineGroups(lineIndices: readonly number[]): readonly EditorLineGroup[] {
	const groups: EditorLineGroup[] = [];
	for (const lineIndex of lineIndices) {
		const previous = groups.at(-1);
		if (previous && lineIndex === previous.endLineIndex + 1) {
			groups[groups.length - 1] = Object.freeze({ ...previous, endLineIndex: lineIndex });
		} else {
			groups.push(Object.freeze({ startLineIndex: lineIndex, endLineIndex: lineIndex }));
		}
	}
	return Object.freeze(groups);
}

function mapOffsetThroughEdits(offset: number, edits: readonly OffsetEdit[]): number {
	let delta = 0;
	for (const edit of edits) {
		if (offset < edit.startOffset) break;
		if (edit.startOffset === edit.endOffset && offset === edit.startOffset) {
			return offset + delta + edit.text.length;
		}
		if (offset <= edit.endOffset) {
			return edit.startOffset + delta + Math.min(offset - edit.startOffset, edit.text.length);
		}
		delta += edit.text.length - (edit.endOffset - edit.startOffset);
	}
	return offset + delta;
}

function movePosition(position: Position, groups: readonly EditorLineGroup[], direction: EditorLineMoveDirection): Position {
	const lineIndex = position.lineNumber - 1;
	const group = groups.find(candidate => lineIndex >= candidate.startLineIndex && lineIndex <= candidate.endLineIndex);
	if (!group) return position;
	return new Position(position.lineNumber + (direction === EditorLineMoveDirection.Up ? -1 : 1), position.column);
}

function applyOffsetEdits(text: string, edits: readonly OffsetEdit[]): string {
	let result = text;
	for (let index = edits.length - 1; index >= 0; index -= 1) {
		const edit = edits[index]!;
		result = result.slice(0, edit.startOffset) + edit.text + result.slice(edit.endOffset);
	}
	return result;
}

function offsetInText(text: string, position: Position): number {
	let lineIndex = 0;
	let offset = 0;
	while (lineIndex < position.lineNumber - 1) {
		const next = text.indexOf("\n", offset);
		if (next < 0) throw new RangeError("Moved line position is outside the result text");
		offset = next + 1;
		lineIndex += 1;
	}
	const lineEnd = text.indexOf("\n", offset);
	const length = (lineEnd < 0 ? text.length : lineEnd) - offset;
	if (position.column < 1 || position.column > length + 1) throw new RangeError("Moved line position exceeds its result line");
	return offset + position.column - 1;
}

registerEditorAction(CopyLinesUpAction);
registerEditorAction(CopyLinesDownAction);
registerEditorAction(DuplicateSelectionAction);
registerEditorAction(MoveLinesUpAction);
registerEditorAction(MoveLinesDownAction);
registerEditorAction(SortLinesAscendingAction);
registerEditorAction(SortLinesDescendingAction);
registerEditorAction(TransposeAction);

registerTextEditorCapabilityContribution({
	id: linesOperationsContributionId,
	commands: [
		'editor.action.indentLines',
		'editor.action.outdentLines',
		'editor.action.deleteLines',
		'editor.action.insertLineBefore',
		'editor.action.insertLineAfter',
		'editor.action.moveLinesUpAction',
		'editor.action.moveLinesDownAction',
		'editor.action.copyLinesUpAction',
		'editor.action.copyLinesDownAction',
		'editor.action.joinLines',
		'editor.action.transpose',
	].map(id => ({ id, canTriggerInlineEdits: true })),
	runtime: {
		descriptor: new ServiceConstructionDescriptor(LinesOperationsContribution),
		instantiation: EditorContributionInstantiation.Eager,
	},
});

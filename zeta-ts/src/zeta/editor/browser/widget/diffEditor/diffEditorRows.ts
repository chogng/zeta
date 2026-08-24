import { fragment as createFragment, h, reset } from '../../../../base/browser/dom.js';
import { type DiffModel } from '../../../common/diff/diffModel.js';
import { LineDiffKind, type DiffRange, type LineDiffRow } from '../../../common/diff/lineDiff.js';

/** Creates one side-by-side row shared by the single- and multi-diff widgets. */
export function createDiffEditorRow(ownerDocument: Document, row: LineDiffRow, model: DiffModel, lineHeight: number, active: boolean, showInlineChanges: boolean): HTMLDivElement {
	const element = h(ownerDocument, 'div');
	element.className = `stanza-diff-editor-row ${row.kind}`;
	element.classList.toggle('active', active);
	element.style.height = `${lineHeight}px`;
	element.style.lineHeight = `${lineHeight}px`;
	element.append(
		createDiffCell(ownerDocument, 'original', row.kind, row.originalLineIndex, row.originalLineIndex === undefined ? undefined : model.original.getLineContent(row.originalLineIndex), row.originalChanges, showInlineChanges),
		createDiffCell(ownerDocument, 'modified', row.kind, row.modifiedLineIndex, row.modifiedLineIndex === undefined ? undefined : model.modified.getLineContent(row.modifiedLineIndex), row.modifiedChanges, showInlineChanges),
	);
	return element;
}

function createDiffCell(ownerDocument: Document, side: 'original' | 'modified', kind: LineDiffKind, lineIndex: number | undefined, text: string | undefined, changes: readonly DiffRange[], showInlineChanges: boolean): HTMLDivElement {
	const cell = h(ownerDocument, 'div');
	cell.className = `stanza-diff-editor-cell ${side}`;
	const number = h(ownerDocument, 'span');
	number.className = 'stanza-diff-editor-line-number';
	number.textContent = lineIndex === undefined ? '' : String(lineIndex + 1);
	const content = h(ownerDocument, 'span');
	content.className = 'stanza-diff-editor-line-content';
	if (text === undefined) {
		cell.classList.add('missing');
	} else {
		if (showInlineChanges) projectDiffText(ownerDocument, content, text, changes, side === 'original' ? LineDiffKind.Removed : LineDiffKind.Added);
		else content.textContent = text;
		if (kind === LineDiffKind.Modified) cell.classList.add(side === 'original' ? 'removed' : 'added');
		else if (kind === LineDiffKind.Removed && side === 'original') cell.classList.add('removed');
		else if (kind === LineDiffKind.Added && side === 'modified') cell.classList.add('added');
	}
	cell.append(number, content);
	return cell;
}

function projectDiffText(ownerDocument: Document, target: HTMLElement, text: string, changes: readonly DiffRange[], changedKind: LineDiffKind): void {
	const fragment = createFragment(ownerDocument);
	let previousEnd = 0;
	for (const change of changes) {
		fragment.append(text.slice(previousEnd, change.startColumn));
		const changed = h(ownerDocument, 'span');
		changed.className = `stanza-diff-editor-inline ${changedKind}`;
		changed.textContent = text.slice(change.startColumn, change.endColumn);
		fragment.append(changed);
		previousEnd = change.endColumn;
	}
	fragment.append(text.slice(previousEnd));
	reset(target, fragment);
}

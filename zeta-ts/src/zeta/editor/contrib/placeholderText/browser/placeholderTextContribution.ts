import { h } from '../../../../base/browser/dom.js';
import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { observableCodeEditor } from '../../../browser/observableCodeEditor.js';
import { type CodeEditorWidget } from '../../../browser/widget/codeEditor/codeEditorWidget.js';
import { type CodeEditorContributionContext } from '../../../browser/widget/codeEditor/codeEditorContributions.js';
import { TextPosition } from '../../../common/core/text.js';

/** Uses the editor option to present placeholder text while the model is empty. */
export class PlaceholderTextContribution extends Disposable {
	public static get(editor: CodeEditorWidget): PlaceholderTextContribution {
		return editor.getContribution<PlaceholderTextContribution>(PlaceholderTextContribution.ID)!;
	}

	public static readonly ID = 'editor.contrib.placeholderText';

	private readonly element: HTMLDivElement | undefined;

	constructor(context: CodeEditorContributionContext) {
		super();
		const placeholder = context.placeholder;
		if (!placeholder) return;

		const editor = observableCodeEditor(context.editor);
		const element = h(context.viewport.element.ownerDocument, 'div', {
			className: ['stanza-editor-placeholder-text', 'stanza-editor-overlay-widget'],
			attributes: { 'aria-hidden': 'true' },
		}, placeholder);
		this.element = element;
		context.viewport.element.append(element);
		this._register(toDisposable(() => element.remove()));
		this._register(editor.valueIsEmpty.onDidChange(empty => this.updateVisibility(empty)));
		this._register(context.viewport.onDidChangeLayout(() => this.updateLayout(context)));
		this.updateVisibility(editor.valueIsEmpty.get());
		this.updateLayout(context);
	}

	private updateVisibility(isEmpty: boolean): void {
		if (this.element) this.element.style.display = isEmpty ? 'block' : 'none';
	}

	private updateLayout(context: CodeEditorContributionContext): void {
		if (!this.element) return;
		const position = context.viewport.getPositionContentCoordinates(TextPosition.at(0, 0));
		this.element.style.left = `${position.left}px`;
		this.element.style.top = `${position.top}px`;
		this.element.style.width = `${Math.max(0, context.viewport.currentLayout.viewportSize.width - position.left)}px`;
		this.element.style.lineHeight = `${position.height}px`;
	}
}

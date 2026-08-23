import { type IDimension } from "../../../../base/browser/geometry.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition } from "../../../common/core/text.js";
import { TextModel } from "../../../common/model/textModel.js";
import { CodeEditorWidget } from "../../../browser/widget/codeEditor/codeEditorWidget.js";
import { type EmbeddedTextEditorOptions, type IEmbeddedTextEditor, type IEmbeddedTextEditorFactory } from "../../../browser/widget/embeddedTextEditor.js";

/**
 * Academic-owned implementation of a line-backed code block.
 *
 * It deliberately composes the line model and browser widget directly instead
 * of mounting the Code mode pane or its contribution bundle.
 */
export class AcademicCodeBlockEditorFactory implements IEmbeddedTextEditorFactory {
	create(options: EmbeddedTextEditorOptions): IEmbeddedTextEditor {
		return new AcademicCodeBlockEditor(options);
	}
}

class AcademicCodeBlockEditor extends DisposableOwner implements IEmbeddedTextEditor {
	private readonly changeEmitter = this.own(new Emitter<string>());
	readonly onDidChange = this.changeEmitter.event;
	private readonly model: TextModel;
	private readonly selections: EditorSelectionController;
	private widget: CodeEditorWidget | undefined;
	private dimension: IDimension = { width: 0, height: 0 };

	constructor(private readonly options: EmbeddedTextEditorOptions) {
		super();
		this.model = this.own(new TextModel(options.initialText));
		this.selections = this.own(new EditorSelectionController(
			this.model,
			TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
			{ readOnly: options.readOnly },
		));
		this.own(this.model.onDidChange(() => this.changeEmitter.fire(this.model.getText())));
	}

	create(parent: HTMLElement): void {
		if (this.widget) throw new ReferenceError("Academic code block editor has already been created");
		this.widget = this.own(new CodeEditorWidget({
			container: parent,
			model: this.model,
			selectionController: this.selections,
			lineHeight: 20,
			ariaLabel: this.options.label,
			viewport: {
				presentation: "embedded",
				showLineNumbers: true,
				activeLineHighlight: "on",
			},
		}));
		this.widget.layout(this.dimension);
	}

	setValue(value: string): void {
		if (this.model.getText() !== value) this.model.reset(value);
	}

	getValue(): string {
		return this.model.getText();
	}

	layout(dimension: IDimension): void {
		this.dimension = {
			width: Math.max(0, dimension.width),
			height: Math.max(0, dimension.height),
		};
		this.widget?.layout(this.dimension);
	}

	focus(): void {
		this.widget?.focus();
	}
}

import "./media/peekViewWidget.css";
import { type Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";
import { type ICodeEditor } from "../../../browser/editorBrowser.js";
import { h } from "../../../../base/browser/dom.js";
import { ZoneWidget } from "../../zoneWidget/browser/zoneWidget.js";

const DEFAULT_PEEK_HEIGHT_IN_LINES = 18;

/** A preview surface anchored in reserved editor space. */
export class EditorPeekViewWidget extends ZoneWidget {
	private body: HTMLDivElement | undefined;

	constructor(editor: ICodeEditor, private readonly initialPosition: Position, private readonly title = "Preview") {
		editor.getModel()?.validatePosition(initialPosition);
		super(editor, {
			className: "stanza-editor-peek-view",
			isAccessible: true,
			isResizeable: true,
			keepEditorSelection: true,
		});
		this.create();
	}

	public get element(): HTMLElement {
		return this.domNode;
	}

	public setBody(content: Node): void {
		this.body!.replaceChildren(content);
	}

	public override show(rangeOrPosition: Range | Position = this.initialPosition, heightInLines = DEFAULT_PEEK_HEIGHT_IN_LINES): void {
		super.show(rangeOrPosition, heightInLines);
	}

	protected override _fillContainer(container: HTMLElement): void {
		const header = h(container.ownerDocument, "header");
		header.className = "stanza-editor-peek-view-header";
		header.textContent = this.title;
		this.body = h(container.ownerDocument, "div");
		this.body.className = "stanza-editor-peek-view-body";
		container.append(header, this.body);
	}

	protected override _doLayout(_heightInPixels: number, _widthInPixels: number): void {}
}

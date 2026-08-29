import "./media/peekView.css";
import { TextRange, type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view.js";
import { h } from "../../../../base/browser/dom.js";
import { ZoneWidget } from "../../zoneWidget/browser/zoneWidget.js";

const DEFAULT_PEEK_HEIGHT_IN_LINES = 18;

/** A preview surface anchored in reserved editor space. */
export class PeekViewWidget extends ZoneWidget {
	private body: HTMLDivElement | undefined;

	constructor(viewport: EditorViewport, private readonly initialPosition: TextPosition, private readonly title = "Preview") {
		viewport.textModel.offsetAt(initialPosition);
		super({
			viewport,
			revealRange: range => viewport.revealPosition(range.start),
		}, {
			className: "stanza-editor-peek-view",
			isAccessible: true,
			isResizable: true,
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

	public override show(rangeOrPosition: TextRange | TextPosition = this.initialPosition, heightInLines = DEFAULT_PEEK_HEIGHT_IN_LINES): void {
		super.show(rangeOrPosition, heightInLines);
	}

	protected override fillContainer(container: HTMLElement): void {
		const header = h(container.ownerDocument, "header");
		header.className = "stanza-editor-peek-view-header";
		header.textContent = this.title;
		this.body = h(container.ownerDocument, "div");
		this.body.className = "stanza-editor-peek-view-body";
		container.append(header, this.body);
	}

	protected override layoutContent(_heightInPixels: number, _widthInPixels: number): void {}
}

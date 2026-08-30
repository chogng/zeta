import './gpuMark.css';
import { reset } from '../../../../base/browser/dom.js';
import { EditorDynamicViewOverlay } from '../../view/editorDynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';
import { ViewPartRows } from '../../view/viewLayer.js';

/** Shows which visible rows were emitted by the GPU text strategy. */
export class StyledGpuMarkOverlay extends EditorDynamicViewOverlay {
	public static readonly CLASS_NAME = 'gpu-mark';
	public readonly domNode: HTMLElement;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, private readonly readGpuLineIndexes: () => ReadonlySet<number>) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-gpu-mark-layer', StyledGpuMarkOverlay.CLASS_NAME));
		this.domNode = this.rows.domNode;
	}

	public render(context: EditorRenderingContext): void {
		const gpuLineIndexes = this.readGpuLineIndexes();
		for (const [lineIndex, row] of this.rows.render(context)) {
			row.classList.toggle('active', gpuLineIndexes.has(lineIndex));
			reset(row);
		}
	}
}

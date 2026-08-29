import './gpuMark.css';
import { reset } from '../../../../base/browser/dom.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type EditorRenderingContext, EditorViewContext } from '../../view/viewPart.js';
import { ViewPartRows } from '../../view/viewLayer.js';

/** Shows which visible rows were emitted by the GPU text strategy. */
export class GpuMarkOverlay extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, private readonly readGpuLineIndexes: () => ReadonlySet<number>) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-gpu-mark-layer', 'stanza-editor-gpu-mark'));
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

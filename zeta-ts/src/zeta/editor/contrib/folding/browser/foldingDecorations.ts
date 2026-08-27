import './media/folding.css';
import { register } from '../../../../base/common/icon.js';
import { lxiconsLibrary } from '../../../../base/common/lxiconsLibrary.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { TextRange } from '../../../common/core/text.js';
import { TextDecorationCollection, type TextDecorationId } from '../../../common/model/decorationCollection.js';
import { TrackedRangeStickiness } from '../../../common/model/trackedRange.js';
import { createStanzaDecorationSource, DecorationPresentation, GlyphMarginLane, type DecorationPresentationResolution, type DecorationSource, type OwnedDecorationSource } from '../../../browser/viewparts/decorations/decorationPresentation.js';
import { EditorFoldingRangeSource, type EditorFoldingRegion } from './foldingRanges.js';
import { type EditorFoldingModel } from './foldingModel.js';

export const foldingExpandedIcon = register('folding-expanded', lxiconsLibrary.chevronDown);
export const foldingCollapsedIcon = register('folding-collapsed', lxiconsLibrary.chevronRight);

const FOLDING_DECORATION_OWNER = 'folding';

/** Owns folding model decorations while the shared glyph-margin part owns their DOM. */
export class FoldingDecorationProvider extends Disposable implements OwnedDecorationSource {
	private readonly collection: TextDecorationCollection<EditorFoldingRegion>;
	private readonly source: DecorationSource;
	private decorationIds: readonly TextDecorationId[] = Object.freeze([]);

	public readonly onDidChange;
	public readonly glyphMarginLanes;

	constructor(private readonly folding: EditorFoldingModel) {
		super();
		this.collection = this._register(new TextDecorationCollection(folding.model));
		this.source = createStanzaDecorationSource(
			this.collection,
			decoration => foldingDecoration(decoration.metadata),
			undefined,
			{ glyphMarginLanes: [{ owner: FOLDING_DECORATION_OWNER, lane: GlyphMarginLane.Center }] },
		);
		this.onDidChange = this.source.onDidChange;
		this.glyphMarginLanes = this.source.glyphMarginLanes;
		this.updateDecorations();
		this._register(folding.onDidChange(() => this.updateDecorations()));
	}

	public get decorations() {
		return this.source.decorations;
	}

	private updateDecorations(): void {
		this.decorationIds = this.collection.deltaDecorations(this.decorationIds, this.folding.regions.map(region => ({
			range: TextRange.emptyAt({ lineIndex: region.startLineIndex, columnIndex: 0 }),
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: region,
		})));
	}
}

function foldingDecoration(region: EditorFoldingRegion): DecorationPresentationResolution {
	const isCollapsed = region.collapsed;
	return Object.freeze({
		presentation: DecorationPresentation.GlyphMargin,
		glyphMargin: {
			owner: FOLDING_DECORATION_OWNER,
			lane: GlyphMarginLane.Center,
			icon: isCollapsed ? foldingCollapsedIcon : foldingExpandedIcon,
			className: region.source === EditorFoldingRangeSource.Manual ? 'stanza-editor-fold-toggle manual' : 'stanza-editor-fold-toggle',
			ariaLabel: isCollapsed ? 'Expand folded lines' : 'Collapse lines',
			expanded: !isCollapsed,
		},
		overviewRuler: false,
		minimap: false,
	});
}

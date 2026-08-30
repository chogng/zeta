import './folding.css';
import { register } from '../../../../base/common/icon.js';
import { lxiconsLibrary } from '../../../../base/common/lxiconsLibrary.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { Range } from '../../../common/core/range.js';
import { TextDecorationCollection, type TextDecorationId } from '../../../common/model/decorationCollection.js';

import { createStanzaDecorationSource, DecorationPresentation, type DecorationPresentationResolution, type DecorationSource, type OwnedDecorationSource } from '../../../browser/viewparts/decorations/decorations.js';
import { EditorFoldingRangeSource, type EditorFoldingRegion } from './foldingRanges.js';
import { type EditorFoldingModel } from './foldingModel.js';
import { TrackedRangeStickiness } from '../../../common/model.js';

export const foldingExpandedIcon = register('folding-expanded', lxiconsLibrary.chevronDown);
export const foldingCollapsedIcon = register('folding-collapsed', lxiconsLibrary.chevronRight);

const FOLDING_DECORATION_OWNER = 'folding';

/** Owns folding model decorations while the shared line-decoration part owns their DOM. */
export class FoldingDecorationProvider extends Disposable implements OwnedDecorationSource {
	private readonly collection: TextDecorationCollection<EditorFoldingRegion>;
	private readonly source: DecorationSource;
	private decorationIds: readonly TextDecorationId[] = Object.freeze([]);

	public readonly onDidChange;
	public readonly glyphMarginLanes;
	public readonly linesDecorationLanes;

	constructor(private readonly folding: EditorFoldingModel) {
		super();
		this.collection = this._register(new TextDecorationCollection(folding.model));
		this.source = createStanzaDecorationSource(
			this.collection,
			decoration => foldingDecoration(decoration.metadata),
			undefined,
			{ linesDecorationLanes: [{ owner: FOLDING_DECORATION_OWNER, width: 20 }] },
		);
		this.onDidChange = this.source.onDidChange;
		this.glyphMarginLanes = this.source.glyphMarginLanes;
		this.linesDecorationLanes = this.source.linesDecorationLanes;
		this.updateDecorations();
		this._register(folding.onDidChange(() => this.updateDecorations()));
	}

	public get decorations() {
		return this.source.decorations;
	}

	private updateDecorations(): void {
		this.decorationIds = this.collection.deltaDecorations(this.decorationIds, this.folding.regions.map(region => ({
			range: Range.fromPositions({ lineNumber: region.startLineIndex + 1, column: 1 }),
			stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
			metadata: region,
		})));
	}
}

function foldingDecoration(region: EditorFoldingRegion): DecorationPresentationResolution {
	const isCollapsed = region.collapsed;
	return Object.freeze({
		presentation: DecorationPresentation.LineDecoration,
		linesDecoration: {
			owner: FOLDING_DECORATION_OWNER,
			icon: isCollapsed ? foldingCollapsedIcon : foldingExpandedIcon,
			className: region.source === EditorFoldingRangeSource.Manual ? 'stanza-editor-fold-toggle manual' : 'stanza-editor-fold-toggle',
			ariaLabel: isCollapsed ? 'Expand folded lines' : 'Collapse lines',
			tooltip: isCollapsed ? 'Expand folded lines' : 'Collapse lines',
			expanded: !isCollapsed,
		},
		overviewRuler: false,
		minimap: false,
	});
}

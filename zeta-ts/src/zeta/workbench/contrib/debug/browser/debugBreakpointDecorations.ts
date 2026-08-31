import './media/debugBreakpointDecorations.css';
import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { register } from '../../../../base/common/icon.js';
import { lxiconsLibrary } from '../../../../base/common/lxiconsLibrary.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { type URI } from '../../../../base/common/uri.js';
import { MouseTargetFactory, MouseTargetKind } from '../../../../editor/browser/controller/mouseTarget.js';
import { type TextEditorContributionContext } from '../../../../editor/browser/editorExtensions.js';
import { createStanzaDecorationSource, DecorationPresentation, type DecorationPresentationResolution, type DecorationSource, type OwnedDecorationSource } from '../../../../editor/browser/viewParts/decorations/decorations.js';
import { Position } from '../../../../editor/common/core/position.js';
import { Range } from '../../../../editor/common/core/range.js';
import { TextDecorationCollection, type TextDecorationId } from '../../../../editor/common/model/decorationCollection.js';
import { type TextModel } from '../../../../editor/common/model/textModel.js';

import { isRemoteResource } from '../../../../platform/remote/common/remote.js';
import { type IDebugBreakpoint, type IDebugService } from '../../../services/debug/common/debugService.js';
import { TrackedRangeStickiness, GlyphMarginLane } from '../../../../editor/common/model.js';

const DEBUG_BREAKPOINT_OWNER = 'debug-breakpoint';
const breakpointIcon = register('breakpoint', lxiconsLibrary.target);

/** Owns breakpoint model decorations while the shared glyph-margin part owns their DOM. */
export class DebugBreakpointDecorationProvider extends Disposable implements OwnedDecorationSource {
	private readonly collection: TextDecorationCollection<IDebugBreakpoint>;
	private readonly source: DecorationSource;
	private decorationIds: readonly TextDecorationId[] = Object.freeze([]);

	public readonly onDidChange;
	public readonly glyphMarginLanes;
	public readonly linesDecorationLanes;

	constructor(private readonly debug: IDebugService, private readonly resource: URI, model: TextModel) {
		super();
		this.collection = this._register(new TextDecorationCollection(model));
		this.source = createStanzaDecorationSource(
			this.collection,
			decoration => breakpointDecoration(decoration.metadata),
			undefined,
			{ glyphMarginLanes: [{ owner: DEBUG_BREAKPOINT_OWNER, lane: GlyphMarginLane.Left }] },
		);
		this.onDidChange = this.source.onDidChange;
		this.glyphMarginLanes = this.source.glyphMarginLanes;
		this.linesDecorationLanes = this.source.linesDecorationLanes;
		this.updateDecorations();
		this._register(debug.onDidChangeBreakpoints(() => this.updateDecorations()));
	}

	public get decorations() {
		return this.source.decorations;
	}

	private updateDecorations(): void {
		const model = this.collection.textModel;
		const breakpoints = this.debug.breakpoints.filter(candidate => candidate.resource.toString() === this.resource.toString() && candidate.lineNumber >= 1 && candidate.lineNumber <= model.lineCount);
		this.decorationIds = this.collection.deltaDecorations(this.decorationIds, breakpoints.map(breakpoint => ({
			range: Range.fromPositions(new Position(breakpoint.lineNumber, 1)),
			stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
			metadata: breakpoint,
		})));
	}
}

/** Routes empty-lane and existing-breakpoint pointer targets to Debug state. */
export class DebugBreakpointController extends Disposable {
	private readonly mouseTargets: MouseTargetFactory;

	constructor(context: TextEditorContributionContext, private readonly debug: IDebugService) {
		super();
		this.mouseTargets = new MouseTargetFactory(context.viewport);
		const resource = context.options.input.resource;
		if (resource.scheme !== 'file' && !isRemoteResource(resource)) return;
		this._register(addDisposableListener(context.viewport.element, 'pointerdown', event => {
			const target = this.mouseTargets.create(event);
			if (target?.kind !== MouseTargetKind.GutterDecoration || target.glyphMarginLane !== GlyphMarginLane.Left) return;
			const lineNumber = target.editorTarget?.position.lineNumber;
			if (lineNumber === undefined) return;
			stopEvent(event);
			context.viewport.element.focus({ preventScroll: true });
			this.debug.toggleBreakpoint(resource, lineNumber);
		}, true));
	}
}

function breakpointDecoration(breakpoint: IDebugBreakpoint): DecorationPresentationResolution {
	const label = `Remove breakpoint at line ${breakpoint.lineNumber}`;
	return Object.freeze({
		presentation: DecorationPresentation.GlyphMargin,
		glyphMargin: {
			owner: DEBUG_BREAKPOINT_OWNER,
			lane: GlyphMarginLane.Left,
			icon: breakpointIcon,
			className: ['zeta-debug-breakpoint-gutter', breakpoint.enabled ? 'enabled' : 'disabled', breakpoint.verified ? 'verified' : 'unverified'].join(' '),
			ariaLabel: label,
			title: breakpoint.message ?? label,
			pressed: true,
		},
		overviewRuler: false,
		minimap: false,
	});
}

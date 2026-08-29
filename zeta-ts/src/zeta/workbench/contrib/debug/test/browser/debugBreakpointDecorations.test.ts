import assert from 'node:assert/strict';
import test from 'node:test';
import { Emitter } from '../../../../../base/common/event.js';
import { Disposable } from '../../../../../base/common/lifecycle.js';
import { URI } from '../../../../../base/common/uri.js';
import { GlyphMarginLane } from '../../../../../editor/browser/viewparts/decorations/decorations.js';
import { TextModel } from '../../../../../editor/common/model/textModel.js';
import { type IDebugBreakpoint, type IDebugService } from '../../../../services/debug/common/debugService.js';
import { DebugBreakpointDecorationProvider } from '../../browser/debugBreakpointDecorations.js';

test('Debug breakpoints project semantic glyph-margin decorations', () => {
	const resource = URI.file('C:\\project\\main.ts');
	using model = new TextModel('first\nsecond\nthird');
	using debug = new BreakpointDebugService(resource);
	using decorations = new DebugBreakpointDecorationProvider(debug as unknown as IDebugService, resource, model);

	assert.deepEqual(decorations.glyphMarginLanes, [{ owner: 'debug-breakpoint', lane: GlyphMarginLane.Left }]);
	assert.deepEqual(decorationState(decorations), [{ lineNumber: 2, iconId: 'breakpoint', pressed: true }]);

	debug.setBreakpoints([{ id: 'second', resource, lineNumber: 3, enabled: false, verified: false }]);
	assert.deepEqual(decorationState(decorations), [{ lineNumber: 3, iconId: 'breakpoint', pressed: true }]);
});

function decorationState(provider: DebugBreakpointDecorationProvider): readonly unknown[] {
	return provider.decorations.map(decoration => ({
		lineNumber: decoration.range.getStartPosition().lineNumber,
		iconId: decoration.glyphMargin?.icon?.id,
		pressed: decoration.glyphMargin?.pressed,
	}));
}

class BreakpointDebugService extends Disposable {
	private readonly breakpointEmitter = this._register(new Emitter<readonly IDebugBreakpoint[]>());
	public breakpoints: readonly IDebugBreakpoint[];
	public readonly onDidChangeBreakpoints = this.breakpointEmitter.event;

	constructor(resource: URI) {
		super();
		this.breakpoints = Object.freeze([{ id: 'first', resource, lineNumber: 2, enabled: true, verified: true }]);
	}

	public setBreakpoints(breakpoints: readonly IDebugBreakpoint[]): void {
		this.breakpoints = Object.freeze([...breakpoints]);
		this.breakpointEmitter.fire(this.breakpoints);
	}
}

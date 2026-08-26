import assert from "node:assert/strict";
import test from "node:test";
import { Emitter, type Event } from "../../../base/common/event.js";
import { TextRange } from "../../common/core/text.js";
import { TextModel } from "../../common/model/textModel.js";
import { EditorViewportChangeReason, ViewLayout } from "../../common/viewLayout/viewLayout.js";
import { type EditorViewportLineSource } from "../../common/viewLayout/linesLayout.js";

test("ViewLayout calculates visible and overscan line ranges", () => {
	using model = new TextModel(lines(100));
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
		overscanLineCount: 2,
	});

	viewport.setViewportSize({ width: 300, height: 100 });
	viewport.setContentWidth(500);
	viewport.setScrollPosition({ left: 250, top: 45 });

	assert.deepEqual(viewport.layout, {
		modelVersion: 1,
		lineHeight: 20,
		viewportSize: { width: 300, height: 100 },
		contentSize: { width: 500, height: 2_000 },
		scrollPosition: { left: 200, top: 45 },
		maximumScrollPosition: { left: 200, top: 1_900 },
		visibleLines: {
			startLineIndex: 2,
			endLineIndexExclusive: 8,
		},
		renderLines: {
			startLineIndex: 0,
			endLineIndexExclusive: 10,
		},
		renderTop: 0,
	});

	viewport.setScrollPosition({ left: 0, top: 100_000 });

	assert.deepEqual({
		scrollPosition: viewport.layout.scrollPosition,
		visibleLines: viewport.layout.visibleLines,
		renderLines: viewport.layout.renderLines,
		renderTop: viewport.layout.renderTop,
	}, {
		scrollPosition: { left: 0, top: 1_900 },
		visibleLines: {
			startLineIndex: 95,
			endLineIndexExclusive: 100,
		},
		renderLines: {
			startLineIndex: 93,
			endLineIndexExclusive: 100,
		},
		renderTop: 1_860,
	});
});

test("ViewLayout includes vertical padding in content and row projection", () => {
	using model = new TextModel(lines(10));
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
		overscanLineCount: 2,
		padding: { top: 20, bottom: 10 },
	});

	viewport.setViewportSize({ width: 200, height: 40 });
	assert.deepEqual({
		contentHeight: viewport.layout.contentSize.height,
		visibleLines: viewport.layout.visibleLines,
		renderLines: viewport.layout.renderLines,
		renderTop: viewport.layout.renderTop,
	}, {
		contentHeight: 230,
		visibleLines: { startLineIndex: 0, endLineIndexExclusive: 1 },
		renderLines: { startLineIndex: 0, endLineIndexExclusive: 3 },
		renderTop: 20,
	});

	viewport.setScrollPosition({ left: 0, top: 1_000 });
	assert.deepEqual({
		scrollTop: viewport.layout.scrollPosition.top,
		visibleLines: viewport.layout.visibleLines,
		renderLines: viewport.layout.renderLines,
		renderTop: viewport.layout.renderTop,
	}, {
		scrollTop: 190,
		visibleLines: { startLineIndex: 8, endLineIndexExclusive: 10 },
		renderLines: { startLineIndex: 6, endLineIndexExclusive: 10 },
		renderTop: 140,
	});
});

test("Viewport resize and line-height changes preserve a stable top line", () => {
	using model = new TextModel(lines(100));
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
		overscanLineCount: 2,
	});
	viewport.setViewportSize({ width: 300, height: 100 });
	viewport.setScrollPosition({ left: 0, top: 1_800 });

	viewport.setViewportSize({ width: 300, height: 200 });
	assert.deepEqual({
		top: viewport.layout.scrollPosition.top,
		maximumTop: viewport.layout.maximumScrollPosition.top,
		visibleLines: viewport.layout.visibleLines,
	}, {
		top: 1_800,
		maximumTop: 1_800,
		visibleLines: {
			startLineIndex: 90,
			endLineIndexExclusive: 100,
		},
	});

	viewport.setLineHeight(40);

	assert.deepEqual({
		top: viewport.layout.scrollPosition.top,
		maximumTop: viewport.layout.maximumScrollPosition.top,
		visibleLines: viewport.layout.visibleLines,
		renderLines: viewport.layout.renderLines,
	}, {
		top: 3_600,
		maximumTop: 3_800,
		visibleLines: {
			startLineIndex: 90,
			endLineIndexExclusive: 95,
		},
		renderLines: {
			startLineIndex: 88,
			endLineIndexExclusive: 97,
		},
	});
});

test("Model line changes update layout and clamp scrolling", () => {
	using model = new TextModel(lines(100));
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
		overscanLineCount: 3,
	});
	viewport.setViewportSize({ width: 200, height: 100 });
	viewport.setScrollPosition({ left: 0, top: 1_900 });
	const events: Array<{
		readonly reason: EditorViewportChangeReason;
		readonly modelVersion: number;
		readonly changedVersion: number | undefined;
	}> = [];
	using listener = viewport.onDidChange(change => events.push({
		reason: change.reason,
		modelVersion: change.layout.modelVersion,
		changedVersion: change.modelChange?.version,
	}));
	const end = model.positionAt(model.createSnapshot().length);

	model.applyEdits([{
		range: TextRange.from(model.positionAt(0), end),
		text: "a\nb",
	}]);

	assert.deepEqual({
		layout: viewport.layout,
		events,
	}, {
		layout: {
			modelVersion: 2,
			lineHeight: 20,
			viewportSize: { width: 200, height: 100 },
			contentSize: { width: 200, height: 100 },
			scrollPosition: { left: 0, top: 0 },
			maximumScrollPosition: { left: 0, top: 0 },
			visibleLines: {
				startLineIndex: 0,
				endLineIndexExclusive: 2,
			},
			renderLines: {
				startLineIndex: 0,
				endLineIndexExclusive: 2,
			},
			renderTop: 0,
		},
		events: [{
			reason: EditorViewportChangeReason.Model,
			modelVersion: 2,
			changedVersion: 2,
		}],
	});
});

test("Same-line model changes still advance the viewport model version", () => {
	using model = new TextModel("abc");
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
	});
	viewport.setViewportSize({ width: 100, height: 20 });
	const reasons: EditorViewportChangeReason[] = [];
	using listener = viewport.onDidChange(change => {
		reasons.push(change.reason);
	});

	model.applyEdits([{
		range: TextRange.from(
			model.positionAt(1),
			model.positionAt(2),
		),
		text: "X",
	}]);

	assert.deepEqual({
		modelVersion: viewport.layout.modelVersion,
		reasons,
	}, {
		modelVersion: 2,
		reasons: [EditorViewportChangeReason.Model],
	});
});

test("Viewport virtualizes a caller-owned visual-line source", () => {
	using model = new TextModel("one\ntwo");
	using visualLines = new MutableLineSource(5);
	using viewport = new ViewLayout(model, {
		lineHeight: 10,
		lineSource: visualLines,
	});
	viewport.setViewportSize({ width: 100, height: 20 });
	viewport.setScrollPosition({ left: 0, top: 30 });
	const reasons: EditorViewportChangeReason[] = [];
	using listener = viewport.onDidChange(change => reasons.push(change.reason));

	visualLines.setLineCount(8);

	assert.deepEqual({
		contentHeight: viewport.layout.contentSize.height,
		scrollTop: viewport.layout.scrollPosition.top,
		visibleLines: viewport.layout.visibleLines,
		reasons,
	}, {
		contentHeight: 80,
		scrollTop: 30,
		visibleLines: { startLineIndex: 3, endLineIndexExclusive: 5 },
		reasons: [EditorViewportChangeReason.LineProjection],
	});
});

test("Zero-sized viewports render no lines and setters suppress no-ops", () => {
	using model = new TextModel("");
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
		overscanLineCount: 5,
	});
	const reasons: EditorViewportChangeReason[] = [];
	using listener = viewport.onDidChange(change => {
		reasons.push(change.reason);
	});

	viewport.setViewportSize({ width: 0, height: 0 });
	viewport.setContentWidth(0);
	viewport.setScrollPosition({ left: -10, top: -20 });

	assert.deepEqual({
		visibleLines: viewport.layout.visibleLines,
		renderLines: viewport.layout.renderLines,
		scrollPosition: viewport.layout.scrollPosition,
		reasons,
	}, {
		visibleLines: {
			startLineIndex: 0,
			endLineIndexExclusive: 0,
		},
		renderLines: {
			startLineIndex: 0,
			endLineIndexExclusive: 0,
		},
		scrollPosition: { left: 0, top: 0 },
		reasons: [],
	});
});

test("ViewLayout validates geometry before changing layout", () => {
	using model = new TextModel("a");

	assert.throws(
		() => new ViewLayout(model, { lineHeight: 0 }),
		/lineHeight must be positive/,
	);
	assert.throws(
		() => new ViewLayout(model, {
			lineHeight: 20,
			overscanLineCount: 1.5,
		}),
		/overscanLineCount/,
	);
	assert.throws(
		() => new ViewLayout(model, {
			lineHeight: 20,
			padding: { top: -1, bottom: 0 },
		}),
		/padding.top must be non-negative/,
	);

	using viewport = new ViewLayout(model, {
		lineHeight: 20,
	});
	assert.throws(
		() => viewport.setViewportSize({
			width: Number.NaN,
			height: 100,
		}),
		/viewportSize.width must be finite/,
	);
	assert.throws(
		() => viewport.setContentWidth(-1),
		/contentWidth must be non-negative/,
	);
	assert.throws(
		() => viewport.setScrollPosition({
			left: 0,
			top: Number.POSITIVE_INFINITY,
		}),
		/scrollPosition.top must be finite/,
	);
	assert.deepEqual(viewport.layout.viewportSize, {
		width: 0,
		height: 0,
	});
});

test("Viewport disposal releases its listener without owning the model", () => {
	using model = new TextModel("a");
	using viewport = new ViewLayout(model, {
		lineHeight: 20,
	});
	let viewportChangeCount = 0;
	using listener = viewport.onDidChange(() => {
		viewportChangeCount++;
	});

	viewport.dispose();
	model.applyEdits([{
		range: TextRange.from(
			model.positionAt(1),
			model.positionAt(1),
		),
		text: "b",
	}]);

	assert.deepEqual({
		modelVersion: model.version,
		viewportModelVersion: viewport.layout.modelVersion,
		viewportChangeCount,
	}, {
		modelVersion: 2,
		viewportModelVersion: 1,
		viewportChangeCount: 0,
	});
});

function lines(count: number): string {
	return Array.from({ length: count }, (_, index) =>
		`line ${index}`).join("\n");
}

class MutableLineSource implements EditorViewportLineSource, Disposable {
	private readonly changeEmitter = new Emitter<void>();
	private _lineCount: number;

	constructor(lineCount: number) {
		this._lineCount = lineCount;
	}

	get lineCount(): number {
		return this._lineCount;
	}

	readonly onDidChange: Event<void> = this.changeEmitter.event;

	setLineCount(lineCount: number): void {
		this._lineCount = lineCount;
		this.changeEmitter.fire();
	}

	[Symbol.dispose](): void {
		this.changeEmitter.dispose();
	}
}

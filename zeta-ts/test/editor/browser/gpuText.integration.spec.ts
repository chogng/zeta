import { expect, test, type Page } from '@playwright/test';

const pageErrors = new WeakMap<object, string[]>();

interface GpuEditorState {
	readonly valueRestored: boolean;
	readonly lineNumber: string | null;
	readonly canvasVisible: boolean;
	readonly allRowsUseGpu: boolean;
	readonly longLineWrapped: boolean;
	readonly rowsHaveCanonicalSpacing: boolean;
	readonly hiddenCanvasUsesDisplayNone: boolean;
	readonly glyphMarginTouchesLineNumber: boolean;
	readonly lineNumberTouchesFolding: boolean;
	readonly foldingPrecedesText: boolean;
	readonly punctuationLineAdvanceMatchesDom: boolean;
}

interface ClearedGpuEditorState {
	readonly value: string;
	readonly lineNumber: string | null;
	readonly renderedRowCount: number;
	readonly gpuRowCount: number;
	readonly text: string | null;
}

interface GpuFrameLayeringState {
	readonly hasFrame: boolean;
	readonly everyFrameIsAtomic: boolean;
}

test.beforeEach(async ({ page }) => {
	const errors: string[] = [];
	pageErrors.set(page, errors);
	page.on('pageerror', error => errors.push(error.stack ?? error.message));
});

test.afterEach(async ({ page }) => {
	await page.evaluate(() => window.zetaGpuTextIntegration?.dispose()).catch(() => undefined);
	expect(pageErrors.get(page) ?? []).toEqual([]);
});

test('GPU text keeps wrapped rows disjoint and the gutter in VS Code order', async ({ page }) => {
	await page.goto('/gpuText.html');
	await expect(page.locator('.stanza-editor-gpu-canvas')).toBeVisible();
	await expect(page.locator('.stanza-editor-fold-toggle').first()).toBeVisible();
	await expect.poll(() => gpuEditorState(page)).toEqual(healthyGpuEditorState());

	const input = page.locator('.stanza-editor-input');
	await input.focus();
	await page.evaluate(() => window.zetaGpuTextIntegration.resetGpuFrameTrace());
	await page.keyboard.press('ControlOrMeta+A');
	await page.keyboard.press('Backspace');
	await expect.poll(() => clearedGpuEditorState(page)).toEqual({
		value: '',
		lineNumber: '1',
		renderedRowCount: 1,
		gpuRowCount: 1,
		text: '',
	});
	await expect.poll(() => gpuFrameLayeringState(page)).toEqual({ hasFrame: true, everyFrameIsAtomic: true });

	await page.evaluate(() => window.zetaGpuTextIntegration.resetGpuFrameTrace());
	await page.keyboard.press('ControlOrMeta+z');
	await expect.poll(() => gpuEditorState(page)).toEqual(healthyGpuEditorState());
	await expect.poll(() => gpuFrameLayeringState(page)).toEqual({ hasFrame: true, everyFrameIsAtomic: true });
});

function healthyGpuEditorState(): GpuEditorState {
	return {
		valueRestored: true,
		lineNumber: '1',
		canvasVisible: true,
		allRowsUseGpu: true,
		longLineWrapped: true,
		rowsHaveCanonicalSpacing: true,
		hiddenCanvasUsesDisplayNone: true,
		glyphMarginTouchesLineNumber: true,
		lineNumberTouchesFolding: true,
		foldingPrecedesText: true,
		punctuationLineAdvanceMatchesDom: true,
	};
}

async function gpuEditorState(page: Page): Promise<GpuEditorState> {
	return page.evaluate(() => {
		const requireElement = <T extends Element = HTMLElement>(root: ParentNode, selector: string): T => {
			const element = root.querySelector<T>(selector);
			if (!element) throw new Error(`Missing GPU editor element '${selector}'`);
			return element;
		};
		const editor = requireElement(document, '.stanza-editor');
		const canvas = requireElement<HTMLCanvasElement>(document, '.stanza-editor-gpu-canvas');
		const firstLine = requireElement(document, '.stanza-editor-line[data-logical-line-index="0"]');
		const glyphMargin = requireElement(document, '.stanza-editor-glyph-margin');
		const lineNumber = requireElement(document, '.stanza-editor-line-margin[data-line-index="0"] .stanza-editor-line-number');
		const folding = requireElement(document, '.stanza-editor-fold-toggle[data-logical-line-index="0"]');
		const text = requireElement(firstLine, '.stanza-editor-line-text');
		const rows = [...editor.querySelectorAll<HTMLElement>('.stanza-editor-line')];
		const rowRectangles = rows.map(row => row.getBoundingClientRect());
		const glyphMarginRectangle = glyphMargin.getBoundingClientRect();
		const lineNumberRectangle = lineNumber.getBoundingClientRect();
		const foldingRectangle = folding.getBoundingClientRect();
		const textRectangle = text.getBoundingClientRect();
		const punctuationLine = rows.find(row => row.textContent === 'console.log(describe(sample));');
		const punctuationText = punctuationLine?.querySelector<HTMLElement>('.stanza-editor-line-text');
		const punctuationRange = document.createRange();
		if (punctuationText) punctuationRange.selectNodeContents(punctuationText);
		const equal = (left: number, right: number) => Math.abs(left - right) < 0.01;
		const canvasWasHidden = canvas.hidden;
		canvas.hidden = true;
		const hiddenCanvasUsesDisplayNone = getComputedStyle(canvas).display === 'none';
		canvas.hidden = canvasWasHidden;
		return {
			valueRestored: window.zetaGpuTextIntegration.getValue() === window.zetaGpuTextIntegration.initialText,
			lineNumber: lineNumber.textContent,
			canvasVisible: !canvas.hidden && getComputedStyle(canvas).display !== 'none',
			allRowsUseGpu: rows.length > 0 && rows.every(row => row.classList.contains('gpu-rendered')),
			longLineWrapped: rows.length > window.zetaGpuTextIntegration.initialText.split('\n').length,
			rowsHaveCanonicalSpacing: rowRectangles.every((rectangle, index) => index === 0 || equal(rectangle.top, rowRectangles[index - 1]!.bottom)),
			hiddenCanvasUsesDisplayNone,
			glyphMarginTouchesLineNumber: equal(glyphMarginRectangle.right, lineNumberRectangle.left),
			lineNumberTouchesFolding: equal(lineNumberRectangle.right, foldingRectangle.left),
			foldingPrecedesText: foldingRectangle.right <= textRectangle.left,
			punctuationLineAdvanceMatchesDom: !!punctuationText && Math.abs(
				window.zetaGpuTextIntegration.measureGpuAdvance(punctuationText.textContent ?? '') - punctuationRange.getBoundingClientRect().width,
			) < 0.1,
		};
	});
}

async function clearedGpuEditorState(page: Page): Promise<ClearedGpuEditorState> {
	return page.evaluate(() => ({
		value: window.zetaGpuTextIntegration.getValue(),
		lineNumber: document.querySelector('.stanza-editor-line-number')?.textContent ?? null,
		renderedRowCount: document.querySelectorAll('.stanza-editor-line').length,
		gpuRowCount: document.querySelectorAll('.stanza-editor-line.gpu-rendered').length,
		text: document.querySelector('.stanza-editor-line-text')?.textContent ?? null,
	}));
}

async function gpuFrameLayeringState(page: Page): Promise<GpuFrameLayeringState> {
	return page.evaluate(() => {
		const passes = window.zetaGpuTextIntegration.readGpuFrameTrace();
		const hasFrame = passes.length >= 2 && passes.length % 2 === 0;
		let everyFrameIsAtomic = hasFrame;
		for (let index = 0; everyFrameIsAtomic && index < passes.length; index += 2) {
			const rectanglePass = passes[index]!;
			const textPass = passes[index + 1]!;
			everyFrameIsAtomic = rectanglePass.label === 'Stanza rectangle pass'
				&& rectanglePass.loadOp === 'clear'
				&& textPass.label === 'Stanza StyledViewLinesGpu pass'
				&& textPass.loadOp === 'load'
				&& rectanglePass.viewId === textPass.viewId
				&& rectanglePass.submissionId === textPass.submissionId;
		}
		return {
			hasFrame,
			everyFrameIsAtomic,
		};
	});
}

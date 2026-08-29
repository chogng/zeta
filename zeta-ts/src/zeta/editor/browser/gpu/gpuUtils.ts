import { fontVariantForCanvas } from '../config/fontMeasurements.js';
import { SemanticTokenModifier, SemanticTokenPresentation, type ResolvedSemanticToken } from '../viewparts/viewLines/viewLine.js';
import { type ITextureAtlasPageGlyph } from './atlas/atlas.js';
import { createContentSegmenter } from './contentSegmenter.js';
import { type GpuRenderFrame, type GpuRenderStrategyInput } from './gpu.js';
import { type GlyphRasterizer } from './raster/glyphRasterizer.js';
import { type IGpuGlyphStyle } from './raster/raster.js';
import { toDisposable, type IDisposable } from '../../../base/common/lifecycle.js';

export const quadVertices = new Float32Array([
	1, 0,
	1, 1,
	0, 1,
	0, 0,
	0, 1,
	1, 0,
]);

export function ensureNonNullable<T>(value: T | null): T {
	if (value === null) throw new Error('Value cannot be null');
	return value;
}

/** Observes the physical canvas size without rounding through CSS pixels. */
export function observeDevicePixelDimensions(element: HTMLElement, ownerWindow: Window, callback: (width: number, height: number) => void): IDisposable {
	const ResizeObserverConstructor = (ownerWindow as Window & { readonly ResizeObserver?: typeof ResizeObserver }).ResizeObserver;
	if (!ResizeObserverConstructor) throw new Error('WebGPU text rendering requires ResizeObserver');
	const observer = new ResizeObserverConstructor((entries: ResizeObserverEntry[]) => {
		const entry = entries.find(candidate => candidate.target === element);
		const size = entry?.devicePixelContentBoxSize?.[0];
		if (!size || size.inlineSize <= 0 || size.blockSize <= 0) return;
		callback(size.inlineSize, size.blockSize);
	});
	observer.observe(element, { box: 'device-pixel-content-box' });
	return toDisposable(() => observer.disconnect());
}

export function validatedDevicePixelRatio(ownerWindow: Window): number {
	const value = ownerWindow.devicePixelRatio;
	if (!Number.isFinite(value) || value <= 0) throw new RangeError('WebGPU device pixel ratio must be finite and positive');
	return value;
}

export function createGpuRenderFrame(glyphRasterizer: GlyphRasterizer, input: GpuRenderStrategyInput, lineIndexes: Iterable<number>): GpuRenderFrame {
		const vertices: number[] = [];
		const gpuLineIndexes = new Set<number>();
		const baseStyle = readBaseStyle(input.rootStyle);
		const tabSize = positiveNumber(Number.parseFloat(input.rootStyle.tabSize), 4);
		for (const visualLineIndex of lineIndexes) {
			const visualLine = input.visualLines.lineAt(visualLineIndex);
			if (!visualLine) continue;
			const text = input.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, visualLine.endColumn);
			const tokens = input.semanticTokenSource?.getLineTokens(visualLine.logicalLineIndex) ?? [];
			const brackets = input.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
			if (!canRenderLine(input, text, tokens)) continue;
			const lineStart = (input.textLeft + (visualLine.wrappedTextIndentWidth ?? 0)) * glyphRasterizer.devicePixelRatio;
			let deviceX = lineStart;
			const lineTop = (input.paddingTop + visualLineIndex * input.layout.lineHeight) * glyphRasterizer.devicePixelRatio;
			const segments = createContentSegmenter(text, { isBasicASCII: /^[\x00-\x7f]*$/u.test(text), useMonospaceOptimizations: false });
			for (let index = 0; index < text.length; index += 1) {
				const segment = segments.getSegmentData(index);
				if (!segment) continue;
				const logicalColumn = visualLine.startColumn + segment.index;
				const style = resolveGlyphStyle(baseStyle, input.rootStyle, tokens, brackets, logicalColumn);
				if (segment.segment === '\t') {
					const space = input.atlas.getGlyph(glyphRasterizer, ' ', style, deviceX);
					const tabStop = Math.max(1, space.advance * tabSize);
					deviceX = lineStart + (Math.floor((deviceX - lineStart) / tabStop) + 1) * tabStop;
					continue;
				}
				const glyph = input.atlas.getGlyph(glyphRasterizer, segment.segment, style, deviceX);
				const fontHeight = glyph.fontBoundingBoxAscent + glyph.fontBoundingBoxDescent;
				const baseline = Math.round(lineTop + Math.floor((input.layout.lineHeight * glyphRasterizer.devicePixelRatio - fontHeight) / 2) + glyph.fontBoundingBoxAscent);
				appendGlyphQuad(vertices, glyph, Math.floor(deviceX) + glyph.originOffsetX, baseline + glyph.originOffsetY);
				deviceX += glyph.advance;
			}
			if (input.visibleLineIndexes.has(visualLineIndex)) gpuLineIndexes.add(visualLineIndex);
		}
		return Object.freeze({ vertices: new Float32Array(vertices), gpuLineIndexes });
	}

function canRenderLine(input: GpuRenderStrategyInput, text: string, tokens: readonly ResolvedSemanticToken[]): boolean {
		if (input.fontLigatures || input.textDirection === 'rtl' || text.length > 2_000 || containsRtl(text)) return false;
		for (const token of tokens) {
			if (token.modifiers?.includes(SemanticTokenModifier.Static) || token.modifiers?.includes(SemanticTokenModifier.Deprecated)) return false;
			if (token.syntaxPresentation?.fontStyle?.some(style => style === 'underline' || style === 'strikethrough')) return false;
		}
		return true;
	}

function readBaseStyle(style: CSSStyleDeclaration): IGpuGlyphStyle {
	return Object.freeze({
		color: style.color,
		fontFamily: style.fontFamily,
		fontSize: positiveNumber(Number.parseFloat(style.fontSize), 14),
		fontStyle: style.fontStyle || 'normal',
		fontVariant: fontVariantForCanvas(style),
		fontWeight: style.fontWeight || '400',
		letterSpacing: style.letterSpacing === 'normal' ? 0 : Number.parseFloat(style.letterSpacing) || 0,
	});
}

function resolveGlyphStyle(base: IGpuGlyphStyle, rootStyle: CSSStyleDeclaration, tokens: readonly ResolvedSemanticToken[], brackets: readonly { readonly startColumn: number; readonly endColumn: number; readonly level: number }[], column: number): IGpuGlyphStyle {
	const token = tokens.find(candidate => candidate.startColumn <= column && candidate.endColumn > column);
	const bracket = brackets.find(candidate => candidate.startColumn <= column && candidate.endColumn > column);
	const syntax = token?.syntaxPresentation;
	const tokenColor = token?.presentation ? cssVariable(rootStyle, tokenColorVariable(token.presentation), base.color) : base.color;
	const bracketColor = bracket ? cssVariable(rootStyle, bracketColorVariable(bracket.level), tokenColor) : tokenColor;
	const fontStyles = syntax?.fontStyle ?? [];
	return Object.freeze({
		...base,
		color: syntax?.foreground ?? bracketColor,
		fontStyle: fontStyles.includes('italic') || token?.modifiers?.some(modifier => modifier === SemanticTokenModifier.Readonly || modifier === SemanticTokenModifier.Abstract || modifier === SemanticTokenModifier.Async) ? 'italic' : base.fontStyle,
		fontWeight: fontStyles.includes('bold') ? 'bold' : token?.modifiers?.includes(SemanticTokenModifier.Declaration) ? '600' : base.fontWeight,
	});
}

function appendGlyphQuad(vertices: number[], glyph: Readonly<ITextureAtlasPageGlyph>, left: number, top: number): void {
	const right = left + glyph.w;
	const bottom = top + glyph.h;
	const atlasRight = glyph.x + glyph.w;
	const atlasBottom = glyph.y + glyph.h;
	vertices.push(
		left, top, glyph.x, glyph.y, glyph.pageIndex,
		right, top, atlasRight, glyph.y, glyph.pageIndex,
		left, bottom, glyph.x, atlasBottom, glyph.pageIndex,
		left, bottom, glyph.x, atlasBottom, glyph.pageIndex,
		right, top, atlasRight, glyph.y, glyph.pageIndex,
		right, bottom, atlasRight, atlasBottom, glyph.pageIndex,
	);
}

function bracketColorVariable(level: number): string {
	const names = ['--zeta-editor-token-keyword-foreground', '--zeta-editor-token-function-foreground', '--zeta-editor-token-type-foreground', '--zeta-editor-token-number-foreground', '--zeta-editor-token-string-foreground', '--zeta-editor-token-variable-foreground'];
	if (!Number.isSafeInteger(level) || level < 1) throw new RangeError('WebGPU bracket level must be a positive integer');
	return names[(level - 1) % names.length]!;
}

function tokenColorVariable(presentation: SemanticTokenPresentation): string {
	switch (presentation) {
		case SemanticTokenPresentation.Comment: return '--zeta-editor-token-comment-foreground';
		case SemanticTokenPresentation.Keyword: return '--zeta-editor-token-keyword-foreground';
		case SemanticTokenPresentation.String: return '--zeta-editor-token-string-foreground';
		case SemanticTokenPresentation.Number: return '--zeta-editor-token-number-foreground';
		case SemanticTokenPresentation.Regexp: return '--zeta-editor-token-regexp-foreground';
		case SemanticTokenPresentation.Type: return '--zeta-editor-token-type-foreground';
		case SemanticTokenPresentation.Function: return '--zeta-editor-token-function-foreground';
		case SemanticTokenPresentation.Variable: return '--zeta-editor-token-variable-foreground';
		case SemanticTokenPresentation.Operator: return '--zeta-editor-token-operator-foreground';
	}
}

function cssVariable(style: CSSStyleDeclaration, name: string, defaultValue: string): string { return style.getPropertyValue(name).trim() || defaultValue; }
function containsRtl(text: string): boolean { return /[\u0590-\u08ff\ufb1d-\ufefc]/u.test(text); }
function positiveNumber(value: number, defaultValue: number): number { return Number.isFinite(value) && value > 0 ? value : defaultValue; }

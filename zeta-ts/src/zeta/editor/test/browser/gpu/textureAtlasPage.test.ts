import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { TextureAtlasPage } from '../../../browser/gpu/atlas/textureAtlasPage.js';
import type { ITextureAtlasAllocator, ITextureAtlasPageGlyph } from '../../../browser/gpu/atlas/atlas.js';
import type { IGlyphRasterizer, IRasterizedGlyph } from '../../../browser/gpu/raster/raster.js';

test('texture atlas keeps cache tuple components distinct', () => {
	const environment = new JSDOM('<!doctype html><body></body>');
	let nextGlyphIndex = 0;
	const allocator: ITextureAtlasAllocator = {
		allocate: () => glyph(nextGlyphIndex++),
		getUsagePreview: async () => new Blob(),
		getStats: () => '',
	};
	const page = new TextureAtlasPage(environment.window.document.body, 0, 64, () => allocator);
	const firstRasterizer = rasterizer('a|b');
	const secondRasterizer = rasterizer('a');
	let rasterizations = 0;
	const rasterize = (): IRasterizedGlyph => {
		rasterizations += 1;
		return {} as IRasterizedGlyph;
	};
	try {
		const first = page.getGlyph(firstRasterizer, 'd', 'c', rasterize);
		const second = page.getGlyph(secondRasterizer, 'd', 'b|c', rasterize);
		assert.notEqual(first, second);
		assert.equal(page.getGlyph(firstRasterizer, 'd', 'c', rasterize), first);
		assert.equal(rasterizations, 2);
		assert.equal(page.version, 2);
	} finally {
		page.dispose();
		environment.window.close();
	}
});

function rasterizer(cacheKey: string): IGlyphRasterizer {
	return {
		id: 0,
		cacheKey,
		devicePixelRatio: 1,
		styleKey: () => '',
		rasterizeGlyph: () => { throw new Error('Unexpected rasterization'); },
		getTextMetrics: () => { throw new Error('Unexpected metrics request'); },
	};
}

function glyph(glyphIndex: number): ITextureAtlasPageGlyph {
	return {
		pageIndex: 0,
		glyphIndex,
		x: glyphIndex,
		y: 0,
		w: 1,
		h: 1,
		originOffsetX: 0,
		originOffsetY: 0,
		advance: 1,
		fontBoundingBoxAscent: 1,
		fontBoundingBoxDescent: 0,
	};
}

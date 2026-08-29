import { GraphemeIterator } from '../../../base/common/strings.js';

export interface IContentSegmenter {
	getSegmentAtIndex(index: number): string | undefined;
	getSegmentData(index: number): Intl.SegmentData | undefined;
}

interface ContentSegmenterOptions {
	readonly isBasicASCII: boolean;
	readonly useMonospaceOptimizations: boolean;
}

export function createContentSegmenter(content: string, options: ContentSegmenterOptions): IContentSegmenter {
	return options.isBasicASCII && options.useMonospaceOptimizations
		? new AsciiContentSegmenter(content)
		: new GraphemeContentSegmenter(content);
}

class AsciiContentSegmenter implements IContentSegmenter {
	constructor(private readonly content: string) {}

	public getSegmentAtIndex(index: number): string | undefined {
		return this.content[index];
	}

	public getSegmentData(_index: number): Intl.SegmentData | undefined {
		return undefined;
	}
}

class GraphemeContentSegmenter implements IContentSegmenter {
	private readonly segments: (Intl.SegmentData | undefined)[] = [];

	constructor(content: string) {
		const iterator = new GraphemeIterator(content);
		while (!iterator.eol()) {
			const index = iterator.offset;
			const segment = content.slice(index, index + iterator.nextGraphemeLength());
			while (this.segments.length < index) {
				this.segments.push(undefined);
			}
			this.segments.push({ segment, index, input: content });
		}
	}

	public getSegmentAtIndex(index: number): string | undefined {
		return this.segments[index]?.segment;
	}

	public getSegmentData(index: number): Intl.SegmentData | undefined {
		return this.segments[index];
	}
}

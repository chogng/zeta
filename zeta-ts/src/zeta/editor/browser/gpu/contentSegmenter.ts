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
		const segmentedContent = [...new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(content)];
		let segmentIndex = 0;
		for (let index = 0; index < content.length; index += 1) {
			const segment = segmentedContent[segmentIndex];
			if (!segment) break;
			if (segment.index !== index) {
				this.segments.push(undefined);
				continue;
			}
			segmentIndex += 1;
			this.segments.push(segment);
		}
	}

	public getSegmentAtIndex(index: number): string | undefined {
		return this.segments[index]?.segment;
	}

	public getSegmentData(index: number): Intl.SegmentData | undefined {
		return this.segments[index];
	}
}

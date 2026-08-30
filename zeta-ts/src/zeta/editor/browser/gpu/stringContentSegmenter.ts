import { safeIntl } from '../../../base/common/date.js';
import type { IContentSegmenter } from './contentSegmenter.js';

export interface StringContentSegmenterOptions {
	readonly isBasicASCII: boolean;
	readonly useMonospaceOptimizations: boolean;
}

export function createStringContentSegmenter(content: string, options: StringContentSegmenterOptions): IContentSegmenter {
	if (options.isBasicASCII && options.useMonospaceOptimizations) {
		return {
			getSegmentAtIndex: index => content[index],
			getSegmentData: () => undefined,
		};
	}

	const segments: (Intl.SegmentData | undefined)[] = [];
	for (const segment of safeIntl.Segmenter(undefined, { granularity: 'grapheme' }).value.segment(content)) {
		while (segments.length < segment.index) {
			segments.push(undefined);
		}
		segments.push(segment);
	}
	return {
		getSegmentAtIndex: index => segments[index]?.segment,
		getSegmentData: index => segments[index],
	};
}

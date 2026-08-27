import { Constants } from '../../../../base/common/uint.js';
import { FloatHorizontalRange } from '../../view/renderingContext.js';
import { DomReadingContext } from './domReadingContext.js';

/** Reads and normalizes browser ranges for rendered line child spans. */
export class RangeUtil {
	private static readonly reusableRanges = new WeakMap<Document, Range>();

	public static readHorizontalRanges(
		domNode: HTMLElement,
		startChildIndex: number,
		startOffset: number,
		endChildIndex: number,
		endOffset: number,
		context: DomReadingContext,
	): FloatHorizontalRange[] | null {
		const maximumChildIndex = domNode.children.length - 1;
		if (maximumChildIndex < 0) return null;
		startChildIndex = Math.min(maximumChildIndex, Math.max(0, startChildIndex));
		endChildIndex = Math.min(maximumChildIndex, Math.max(0, endChildIndex));

		if (startChildIndex === endChildIndex && startOffset === 0 && endOffset === 0 && !domNode.children[startChildIndex]!.firstChild) {
			const clientRectangles = domNode.children[startChildIndex]!.getClientRects();
			context.markDidDomLayout();
			return this.createHorizontalRanges(clientRectangles, context.clientRectDeltaLeft, context.clientRectScale);
		}

		if (startChildIndex !== endChildIndex && endChildIndex > 0 && endOffset === 0) {
			endChildIndex -= 1;
			endOffset = Constants.MAX_SAFE_SMALL_INTEGER;
		}

		let startElement = domNode.children[startChildIndex]!.firstChild;
		let endElement = domNode.children[endChildIndex]!.firstChild;
		if (!startElement && startOffset === 0 && startChildIndex > 0) {
			startElement = domNode.children[startChildIndex - 1]!.firstChild;
			startOffset = Constants.MAX_SAFE_SMALL_INTEGER;
		}
		if (!endElement && endOffset === 0 && endChildIndex > 0) {
			endElement = domNode.children[endChildIndex - 1]!.firstChild;
			endOffset = Constants.MAX_SAFE_SMALL_INTEGER;
		}
		if (!startElement || !endElement) return null;

		startOffset = Math.min(startElement.textContent?.length ?? 0, Math.max(0, startOffset));
		endOffset = Math.min(endElement.textContent?.length ?? 0, Math.max(0, endOffset));
		const clientRectangles = this.readClientRectangles(startElement, startOffset, endElement, endOffset, context.endNode);
		context.markDidDomLayout();
		return this.createHorizontalRanges(clientRectangles, context.clientRectDeltaLeft, context.clientRectScale);
	}

	private static readClientRectangles(startElement: Node, startOffset: number, endElement: Node, endOffset: number, endNode: HTMLElement): DOMRectList | null {
		const range = this.reusableRange(startElement.ownerDocument!);
		try {
			range.setStart(startElement, startOffset);
			range.setEnd(endElement, endOffset);
			return range.getClientRects();
		} catch {
			return null;
		} finally {
			range.selectNodeContents(endNode);
		}
	}

	private static reusableRange(document: Document): Range {
		const existing = this.reusableRanges.get(document);
		if (existing) return existing;
		const range = document.createRange();
		this.reusableRanges.set(document, range);
		return range;
	}

	private static createHorizontalRanges(clientRectangles: DOMRectList | null, originLeft: number, scale: number): FloatHorizontalRange[] | null {
		if (!clientRectangles || clientRectangles.length === 0) return null;
		const ranges: FloatHorizontalRange[] = [];
		for (const rectangle of clientRectangles) {
			if (!Number.isFinite(rectangle.left) || !Number.isFinite(rectangle.width)) continue;
			ranges.push(new FloatHorizontalRange(Math.max(0, (rectangle.left - originLeft) / scale), rectangle.width / scale));
		}
		if (ranges.length === 0) return null;
		ranges.sort(FloatHorizontalRange.compare);
		const result: FloatHorizontalRange[] = [];
		let previous = ranges[0]!;
		for (let index = 1; index < ranges.length; index += 1) {
			const range = ranges[index]!;
			if (previous.left + previous.width + 0.9 >= range.left) {
				previous.width = Math.max(previous.width, range.left + range.width - previous.left);
			} else {
				result.push(previous);
				previous = range;
			}
		}
		result.push(previous);
		return result;
	}
}

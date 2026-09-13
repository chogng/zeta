/**
 * Measures text using the editor's current visual font metrics.
 *
 * The common geometry and navigation algorithms depend only on this small
 * contract. Browser implementations may add lifecycle operations such as
 * refreshing cached metrics, but those operations do not belong in common.
 */
export interface TextMeasurer {
	readonly horizontalPadding: number;
	readonly contentLeftPadding: number;
	measureLineWidth(text: string): number;
}

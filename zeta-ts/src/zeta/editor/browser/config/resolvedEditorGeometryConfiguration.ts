import { EDITOR_FONT_DEFAULTS } from '../../common/config/fontInfo.js';

/** Input accepted by the browser editor's geometry configuration resolver. */
export interface EditorGeometryConfigurationInput {
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly lineHeight?: number;
	readonly fontLigatures?: boolean;
}

/** Resolved values that affect the editor's initial font and line geometry. */
export interface ResolvedEditorGeometryConfiguration {
	readonly fontFamily?: string;
	readonly fontSize: number;
	readonly lineHeight: number;
	readonly fontLigatures: boolean;
}

const DEFAULT_LINE_HEIGHT = 20;

/**
 * Resolves browser geometry options once at the composition boundary.
 *
 * The option contract lives in the browser adapter because these values are
 * currently consumed only while assembling the DOM editor surface.
 */
export function resolveEditorGeometryConfiguration(options: EditorGeometryConfigurationInput): ResolvedEditorGeometryConfiguration {
	if (!options || typeof options !== 'object') {
		throw new TypeError('Editor configuration must be an object');
	}
	if (options.fontFamily !== undefined && (typeof options.fontFamily !== 'string' || !options.fontFamily.trim())) {
		throw new TypeError('Editor font family must be a non-empty string');
	}
	if (options.fontSize !== undefined && (!Number.isSafeInteger(options.fontSize) || options.fontSize < 8 || options.fontSize > 40)) {
		throw new RangeError('Editor font size must be an integer between 8 and 40');
	}
	if (options.lineHeight !== undefined && (!Number.isSafeInteger(options.lineHeight) || options.lineHeight < 12 || options.lineHeight > 80)) {
		throw new RangeError('Editor line height must be an integer between 12 and 80');
	}
	if (options.fontLigatures !== undefined && typeof options.fontLigatures !== 'boolean') {
		throw new TypeError('Editor font ligatures option must be boolean');
	}

	return Object.freeze({
		fontFamily: options.fontFamily,
		fontSize: options.fontSize ?? EDITOR_FONT_DEFAULTS.fontSize,
		lineHeight: options.lineHeight ?? defaultLineHeight(options.fontSize),
		fontLigatures: options.fontLigatures ?? false,
	});
}

function defaultLineHeight(fontSize: number | undefined): number {
	return fontSize === undefined ? DEFAULT_LINE_HEIGHT : Math.max(DEFAULT_LINE_HEIGHT, Math.ceil(fontSize * 1.5));
}

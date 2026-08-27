/** Resolved font values that can be applied to a browser editor root. */
export interface EditorDomFontInfo {
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly fontLigatures: boolean;
}

/** Applies the editor font contract without coupling callers to CSS details. */
export function applyEditorFontInfo(element: HTMLElement, fontInfo: EditorDomFontInfo): void {
	if (fontInfo.fontFamily) element.style.fontFamily = fontInfo.fontFamily;
	if (fontInfo.fontSize !== undefined) element.style.fontSize = `${fontInfo.fontSize}px`;
	element.style.fontVariantLigatures = fontInfo.fontLigatures ? 'normal' : 'none';
}

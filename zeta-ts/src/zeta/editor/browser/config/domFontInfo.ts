import { type EditorConfiguration } from './editorConfiguration.js';

/** Resolved font values that can be applied to a browser editor root. */
export type EditorDomFontInfo = Pick<EditorConfiguration, 'fontFamily' | 'fontSize' | 'fontLigatures'>;

/** Applies the editor font contract without coupling callers to CSS details. */
export function applyEditorFontInfo(element: HTMLElement, fontInfo: EditorDomFontInfo): void {
	if (fontInfo.fontFamily) element.style.fontFamily = fontInfo.fontFamily;
	if (fontInfo.fontSize !== undefined) element.style.fontSize = `${fontInfo.fontSize}px`;
	element.style.fontVariantLigatures = fontInfo.fontLigatures ? 'normal' : 'none';
}

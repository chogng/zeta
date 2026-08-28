import { Emitter, type Event } from '../../../../base/common/event.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { RGBA8 } from '../../../common/core/misc/rgba.js';
import { type LanguageColorPresentation } from '../common/color.js';

/** Owns the editable color and the provider-backed text representation selected for insertion. */
export class ColorPickerModel extends Disposable {
	private readonly changeColorEmitter = this._register(new Emitter<RGBA8>());
	private readonly changePresentationsEmitter = this._register(new Emitter<readonly LanguageColorPresentation[]>());
	private readonly changeSelectedPresentationEmitter = this._register(new Emitter<LanguageColorPresentation | undefined>());
	private currentColor: RGBA8;
	private presentations: readonly LanguageColorPresentation[] = Object.freeze([]);
	private selectedIndex = 0;

	readonly onDidChangeColor: Event<RGBA8> = this.changeColorEmitter.event;
	readonly onDidChangePresentations: Event<readonly LanguageColorPresentation[]> = this.changePresentationsEmitter.event;
	readonly onDidChangeSelectedPresentation: Event<LanguageColorPresentation | undefined> = this.changeSelectedPresentationEmitter.event;

	constructor(readonly originalColor: RGBA8) {
		super();
		this.currentColor = originalColor;
	}

	get color(): RGBA8 {
		return this.currentColor;
	}

	get colorPresentations(): readonly LanguageColorPresentation[] {
		return this.presentations;
	}

	get selectedPresentation(): LanguageColorPresentation | undefined {
		return this.presentations[this.selectedIndex];
	}

	setColor(color: RGBA8): void {
		this.assertNotDisposed();
		if (this.currentColor.equals(color)) return;
		this.currentColor = color;
		this.changeColorEmitter.fire(color);
	}

	setColorPresentations(presentations: readonly LanguageColorPresentation[], originalText?: string): void {
		this.assertNotDisposed();
		this.presentations = Object.freeze([...presentations]);
		this.selectedIndex = selectPresentationIndex(this.presentations, originalText, this.selectedIndex);
		this.changePresentationsEmitter.fire(this.presentations);
		this.changeSelectedPresentationEmitter.fire(this.selectedPresentation);
	}

	selectPresentation(index: number): void {
		this.assertNotDisposed();
		if (!Number.isSafeInteger(index) || index < 0 || index >= this.presentations.length) throw new RangeError('Color presentation index is out of range');
		if (this.selectedIndex === index) return;
		this.selectedIndex = index;
		this.changeSelectedPresentationEmitter.fire(this.selectedPresentation);
	}

	selectNextPresentation(): void {
		this.assertNotDisposed();
		if (this.presentations.length === 0) return;
		this.selectPresentation((this.selectedIndex + 1) % this.presentations.length);
	}
}

function selectPresentationIndex(presentations: readonly LanguageColorPresentation[], originalText: string | undefined, previousIndex: number): number {
	if (presentations.length === 0) return 0;
	if (originalText) {
		const normalized = originalText.trim().toLowerCase();
		const exact = presentations.findIndex(presentation => presentation.label.toLowerCase() === normalized);
		if (exact >= 0) return exact;
		const prefix = normalized.slice(0, normalized.indexOf('(') >= 0 ? normalized.indexOf('(') : normalized.startsWith('#') ? 1 : normalized.length);
		const matchingPrefix = presentations.findIndex(presentation => presentation.label.toLowerCase().startsWith(prefix));
		if (matchingPrefix >= 0) return matchingPrefix;
	}
	return Math.min(previousIndex, presentations.length - 1);
}

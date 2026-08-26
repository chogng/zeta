import type { HoverContent } from '../hover/hover.js';
import { getHoverDelegate, type IManagedHover } from '../hover/hoverDelegate.js';
import { DisposableOwner, DisposableSlot } from '../../../common/lifecycle.js';
import { renderLabelWithIcons } from './iconLabels.js';

/** Lightweight icon-aware text label for controls that do not need descriptions. */
export class SimpleIconLabel extends DisposableOwner {
	private readonly hover = this.own(new DisposableSlot<IManagedHover>());

	constructor(private readonly container: HTMLElement) {
		super();
	}

	set text(value: string) {
		renderLabelWithIcons(this.container, value ?? '');
	}

	set title(value: HoverContent) {
		this.hover.clear();
		this.container.removeAttribute('title');
		if (value === undefined || value === '') return;
		this.hover.replace(getHoverDelegate().setupHover({ target: this.container, content: value }));
	}
}

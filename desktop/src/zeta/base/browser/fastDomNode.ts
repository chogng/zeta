/**
 * Caches inline geometry, visibility, class, and text writes for one retained DOM node.
 * After a setter is first used, that property must be mutated only through this wrapper.
 */
export class FastDomNode<TElement extends HTMLElement> {
	private width: string | undefined;
	private height: string | undefined;
	private top: string | undefined;
	private left: string | undefined;
	private right: string | undefined;
	private bottom: string | undefined;
	private lineHeight: string | undefined;
	private transform: string | undefined;
	private boxShadow: string | undefined;
	private className: string | undefined;
	private textContent: string | undefined;
	private hidden: boolean | undefined;
	private tabIndex: number | undefined;

	constructor(public readonly domNode: TElement) { }

	public setWidth(value: number | string): void {
		const width = numberAsPixels(value);
		if ((this.width ?? this.domNode.style.width) === width) {
			this.width = width;
			return;
		}
		this.width = width;
		this.domNode.style.width = width;
	}

	public setHeight(value: number | string): void {
		const height = numberAsPixels(value);
		if ((this.height ?? this.domNode.style.height) === height) {
			this.height = height;
			return;
		}
		this.height = height;
		this.domNode.style.height = height;
	}

	public setTop(value: number | string): void {
		const top = numberAsPixels(value);
		if ((this.top ?? this.domNode.style.top) === top) {
			this.top = top;
			return;
		}
		this.top = top;
		this.domNode.style.top = top;
	}

	public setLeft(value: number | string): void {
		const left = numberAsPixels(value);
		if ((this.left ?? this.domNode.style.left) === left) {
			this.left = left;
			return;
		}
		this.left = left;
		this.domNode.style.left = left;
	}

	public setRight(value: number | string): void {
		const right = numberAsPixels(value);
		if ((this.right ?? this.domNode.style.right) === right) {
			this.right = right;
			return;
		}
		this.right = right;
		this.domNode.style.right = right;
	}

	public setBottom(value: number | string): void {
		const bottom = numberAsPixels(value);
		if ((this.bottom ?? this.domNode.style.bottom) === bottom) {
			this.bottom = bottom;
			return;
		}
		this.bottom = bottom;
		this.domNode.style.bottom = bottom;
	}

	public setLineHeight(value: number | string): void {
		const lineHeight = numberAsPixels(value);
		if ((this.lineHeight ?? this.domNode.style.lineHeight) === lineHeight) {
			this.lineHeight = lineHeight;
			return;
		}
		this.lineHeight = lineHeight;
		this.domNode.style.lineHeight = lineHeight;
	}

	public setTransform(value: string): void {
		if ((this.transform ?? this.domNode.style.transform) === value) {
			this.transform = value;
			return;
		}
		this.transform = value;
		this.domNode.style.transform = value;
	}

	public setBoxShadow(value: string): void {
		if ((this.boxShadow ?? this.domNode.style.boxShadow) === value) {
			this.boxShadow = value;
			return;
		}
		this.boxShadow = value;
		this.domNode.style.boxShadow = value;
	}

	public setClassName(value: string): void {
		if ((this.className ?? this.domNode.className) === value) {
			this.className = value;
			return;
		}
		this.className = value;
		this.domNode.className = value;
	}

	public setTextContent(value: string): void {
		if ((this.textContent ?? this.domNode.textContent) === value) {
			this.textContent = value;
			return;
		}
		this.textContent = value;
		this.domNode.textContent = value;
	}

	public setHidden(value: boolean): void {
		if ((this.hidden ?? this.domNode.hidden) === value) {
			this.hidden = value;
			return;
		}
		this.hidden = value;
		this.domNode.hidden = value;
	}

	public setTabIndex(value: number): void {
		if ((this.tabIndex ?? this.domNode.tabIndex) === value) {
			this.tabIndex = value;
			return;
		}
		this.tabIndex = value;
		this.domNode.tabIndex = value;
	}
}

function numberAsPixels(value: number | string): string {
	return typeof value === 'number' ? `${value}px` : value;
}

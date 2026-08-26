/** Clipboard data readable by an editor input adapter. */
export interface IReadableClipboardData {
	readonly types: readonly string[];
	readonly files: readonly File[];
	getData(type: string): string;
}

/** Clipboard data writable by an editor input adapter. */
export interface IWritableClipboardData {
	setData(type: string, value: string): void;
}

/** A copy or cut event exposed before the clipboard contribution handles it. */
export interface IClipboardCopyEvent {
	readonly isCut: boolean;
	readonly clipboardData: IWritableClipboardData;
	readonly hasClipboardData: boolean;
	readonly browserEvent: ClipboardEvent;
	setHandled(): void;
	readonly isHandled: boolean;
}

/** A paste event exposed before the clipboard contribution handles it. */
export interface IClipboardPasteEvent {
	readonly clipboardData: IReadableClipboardData;
	readonly text: string;
	readonly browserEvent: ClipboardEvent;
	setHandled(): void;
	readonly isHandled: boolean;
}

export function createClipboardCopyEvent(browserEvent: ClipboardEvent, isCut: boolean): IClipboardCopyEvent {
	let handled = false;
	return {
		isCut,
		clipboardData: createWritableClipboardData(browserEvent.clipboardData),
		hasClipboardData: browserEvent.clipboardData !== null,
		browserEvent,
		setHandled: () => {
			if (handled) return;
			handled = true;
			browserEvent.preventDefault();
			browserEvent.stopImmediatePropagation();
		},
		get isHandled(): boolean {
			return handled;
		},
	};
}

export function createClipboardPasteEvent(browserEvent: ClipboardEvent): IClipboardPasteEvent {
	let handled = false;
	const clipboardData = createReadableClipboardData(browserEvent.clipboardData);
	return {
		clipboardData,
		text: readPlainText(clipboardData),
		browserEvent,
		setHandled: () => {
			if (handled) return;
			handled = true;
			browserEvent.preventDefault();
			browserEvent.stopImmediatePropagation();
		},
		get isHandled(): boolean {
			return handled;
		},
	};
}

export function createReadableClipboardData(dataTransfer: DataTransfer | null | undefined): IReadableClipboardData {
	return {
		types: Object.freeze(Array.from(dataTransfer?.types ?? [])),
		files: Object.freeze(Array.from(dataTransfer?.files ?? [])),
		getData: (type: string): string => {
			try {
				return dataTransfer?.getData(type) ?? '';
			} catch {
				return '';
			}
		},
	};
}

export function createWritableClipboardData(dataTransfer: DataTransfer | null | undefined): IWritableClipboardData {
	return {
		setData: (type: string, value: string): void => {
			dataTransfer?.setData(type, value);
		},
	};
}

function readPlainText(clipboardData: IReadableClipboardData): string {
	return clipboardData.getData('text/plain');
}

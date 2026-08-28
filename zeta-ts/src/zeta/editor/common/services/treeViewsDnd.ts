/** Stores asynchronous tree drag data until one drop target consumes it. */
export interface ITreeViewsDnDService<T> {
	readonly _serviceBrand: undefined;

	removeDragOperationTransfer(identifier: string | undefined): Promise<T | undefined> | undefined;
	addDragOperationTransfer(identifier: string, transferPromise: Promise<T | undefined>): void;
}

/**
 * Coordinates one-renderer tree drag operations without exposing their data
 * through the browser drag payload.
 */
export class TreeViewsDnDService<T> implements ITreeViewsDnDService<T> {
	readonly _serviceBrand = undefined;
	private readonly dragOperations = new Map<string, Promise<T | undefined>>();

	removeDragOperationTransfer(identifier: string | undefined): Promise<T | undefined> | undefined {
		if (!identifier) return undefined;
		const operation = this.dragOperations.get(identifier);
		if (!operation) return undefined;
		this.dragOperations.delete(identifier);
		return operation;
	}

	addDragOperationTransfer(identifier: string, transferPromise: Promise<T | undefined>): void {
		this.dragOperations.set(identifier, transferPromise);
	}
}

/** Identifies tree data retained outside the browser drag payload. */
export class DraggedTreeItemsIdentifier {
	constructor(readonly identifier: string) {}
}

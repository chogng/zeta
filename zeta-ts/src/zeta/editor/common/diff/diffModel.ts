import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type TextModel } from "../model/textModel.js";
import { type DiffComputationDocument, type IDiffComputationService } from "./diffComputationService.js";
import { type LineDiff } from "./lineDiff.js";

export interface DiffModelOptions {
	readonly original: TextModel;
	readonly modified: TextModel;
	readonly computationService: IDiffComputationService;
}

export interface DiffModelLoadingState {
	readonly kind: "loading";
	readonly originalVersion: number;
	readonly modifiedVersion: number;
}

export interface DiffModelReadyState {
	readonly kind: "ready";
	readonly originalVersion: number;
	readonly modifiedVersion: number;
	readonly diff: LineDiff;
}

export interface DiffModelErrorState {
	readonly kind: "error";
	readonly originalVersion: number;
	readonly modifiedVersion: number;
	readonly error: Error;
}

/** The current version-pinned state of one original/modified text comparison. */
export type DiffModelState = DiffModelLoadingState | DiffModelReadyState | DiffModelErrorState;

/**
 * DOM-free derived model for one pair of caller-owned text documents.
 *
 * It owns request cancellation and result validity, but never either source
 * TextModel or the computation service. A result becomes visible only when
 * both source versions still match the request that produced it.
 */
export class DiffModel extends DisposableOwner {
	private readonly changeEmitter = this.own(new Emitter<DiffModelState>());
	private activeRequest: AbortController | undefined;
	private requestGeneration = 0;
	private _state: DiffModelState;

	readonly onDidChange: Event<DiffModelState> = this.changeEmitter.event;

	constructor(private readonly options: DiffModelOptions) {
		super();
		validateOptions(options);
		this._state = Object.freeze({
			kind: "loading",
			originalVersion: options.original.version,
			modifiedVersion: options.modified.version,
		});
		this.own(options.original.onDidChange(() => this.refresh()));
		this.own(options.modified.onDidChange(() => this.refresh()));
		this.defer(() => {
			this.activeRequest?.abort("diffModelDisposed");
			this.activeRequest = undefined;
		});
		this.refresh();
	}

	get original(): TextModel {
		return this.options.original;
	}

	get modified(): TextModel {
		return this.options.modified;
	}

	get state(): DiffModelState {
		return this._state;
	}

	get diff(): LineDiff | undefined {
		return this._state.kind === "ready" ? this._state.diff : undefined;
	}

	/** Starts a fresh computation for the current source-model versions. */
	refresh(): void {
		if (this.isDisposed) return;
		const original = snapshot(this.original);
		const modified = snapshot(this.modified);
		this.activeRequest?.abort("supersededDiffModelRequest");
		const controller = new AbortController();
		this.activeRequest = controller;
		const generation = ++this.requestGeneration;
		this.setState(Object.freeze({
			kind: "loading",
			originalVersion: original.version,
			modifiedVersion: modified.version,
		}));
		void this.compute(generation, controller, original, modified);
	}

	private async compute(generation: number, controller: AbortController, original: DiffComputationDocument, modified: DiffComputationDocument): Promise<void> {
		try {
			const diff = await this.options.computationService.compute(Object.freeze({
				original,
				modified,
			}), controller.signal);
			if (!this.isCurrentRequest(generation, controller, original, modified)) return;
			this.activeRequest = undefined;
			this.setState(Object.freeze({
				kind: "ready",
				originalVersion: original.version,
				modifiedVersion: modified.version,
				diff,
			}));
		} catch (error) {
			if (controller.signal.aborted || !this.isCurrentRequest(generation, controller, original, modified)) return;
			this.activeRequest = undefined;
			this.setState(Object.freeze({
				kind: "error",
				originalVersion: original.version,
				modifiedVersion: modified.version,
				error: asError(error),
			}));
		}
	}

	private isCurrentRequest(generation: number, controller: AbortController, original: DiffComputationDocument, modified: DiffComputationDocument): boolean {
		return !this.isDisposed &&
			this.requestGeneration === generation &&
			this.activeRequest === controller &&
			this.original.version === original.version &&
			this.modified.version === modified.version;
	}

	private setState(state: DiffModelState): void {
		this._state = state;
		this.changeEmitter.fire(state);
	}
}

function snapshot(model: TextModel): DiffComputationDocument {
	const snapshot = model.createSnapshot();
	return Object.freeze({ version: snapshot.version, text: snapshot.getText() });
}

function validateOptions(options: DiffModelOptions): void {
	if (!options || typeof options !== "object" || !options.original || !options.modified) {
		throw new TypeError("Diff model requires original and modified text models");
	}
	if (!options.computationService || typeof options.computationService.compute !== "function") {
		throw new TypeError("Diff model requires a diff computation service");
	}
}

function asError(error: unknown): Error {
	return error instanceof Error ? error : new Error(String(error));
}

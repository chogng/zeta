import { Emitter, type Event } from "../../../base/common/event.js";
import { Disposable, toDisposable } from "../../../base/common/lifecycle.js";
import { TextRange } from "../core/text.js";
import { TextModel } from "./textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "./trackedRange.js";

declare const textDecorationIdBrand: unique symbol;

export type TextDecorationId = number & {
	readonly [textDecorationIdBrand]: "TextDecorationId";
};

export interface TextDecorationSpec<TMetadata> {
	readonly range: TextRange;
	readonly stickiness: TrackedRangeStickiness;
	readonly metadata: TMetadata;
}

export interface TextDecorationSnapshot<TMetadata> {
	readonly id: TextDecorationId;
	readonly range: TextRange;
	readonly metadata: TMetadata;
}

export enum TextDecorationChangeReason {
	Content = "content",
	Range = "range",
}

export interface TextDecorationChange<TMetadata> {
	readonly reason: TextDecorationChangeReason;
	readonly modelVersion: number;
	readonly decorations: readonly TextDecorationSnapshot<TMetadata>[];
}

interface DecorationEntry<TMetadata> {
	readonly id: TextDecorationId;
	readonly trackedRange: TrackedRange;
	readonly metadata: TMetadata;
	lastRange: TextRange;
}

let nextTextDecorationId = 1;

/**
 * One owner's decorations over a shared text model.
 *
 * Metadata is opaque to core and remains caller-owned. Renderer styling,
 * language semantics, diagnostics, and search-match policy stay outside this
 * collection.
 */
export class TextDecorationCollection<TMetadata> extends Disposable {
	private readonly changeEmitter =
		this._register(new Emitter<TextDecorationChange<TMetadata>>());
	private readonly entries =
		new Map<TextDecorationId, DecorationEntry<TMetadata>>();

	readonly onDidChange: Event<TextDecorationChange<TMetadata>> =
		this.changeEmitter.event;

	constructor(private readonly model: TextModel) {
		super();
		this._register(model.onDidChange(() => this.acceptModelChange()));
		this._register(toDisposable(() => {
			for (const entry of this.entries.values()) {
				entry.trackedRange.dispose();
			}
			this.entries.clear();
		}));
	}

	get size(): number {
		this.assertNotDisposed();
		return this.entries.size;
	}

	get textModel(): TextModel {
		this.assertNotDisposed();
		return this.model;
	}

	get decorations(): readonly TextDecorationSnapshot<TMetadata>[] {
		this.assertNotDisposed();
		return this.createSnapshot();
	}

	get(
		id: TextDecorationId,
	): TextDecorationSnapshot<TMetadata> | undefined {
		this.assertNotDisposed();
		const entry = this.entries.get(id);
		return entry ? snapshotEntry(entry) : undefined;
	}

	add(spec: TextDecorationSpec<TMetadata>): TextDecorationId {
		this.assertNotDisposed();
		this.validateSpec(spec);
		const entry = this.createEntry(spec);
		this.entries.set(entry.id, entry);
		this.emitChange(TextDecorationChangeReason.Content);
		return entry.id;
	}

	update(
		id: TextDecorationId,
		spec: TextDecorationSpec<TMetadata>,
	): void {
		this.assertNotDisposed();
		const previous = this.entries.get(id);
		if (!previous) {
			throw new RangeError(`Unknown text decoration ${id}`);
		}
		this.validateSpec(spec);
		const trackedRange = this.model.trackRange(
			spec.range,
			spec.stickiness,
		);
		const next: DecorationEntry<TMetadata> = {
			id,
			trackedRange,
			metadata: spec.metadata,
			lastRange: spec.range,
		};
		this.entries.set(id, next);
		previous.trackedRange.dispose();
		this.emitChange(TextDecorationChangeReason.Content);
	}

	delete(id: TextDecorationId): boolean {
		this.assertNotDisposed();
		const entry = this.entries.get(id);
		if (!entry) return false;
		this.entries.delete(id);
		entry.trackedRange.dispose();
		this.emitChange(TextDecorationChangeReason.Content);
		return true;
	}

	replaceAll(
		specs: readonly TextDecorationSpec<TMetadata>[],
	): readonly TextDecorationId[] {
		this.assertNotDisposed();
		for (const spec of specs) this.validateSpec(spec);
		if (specs.length === 0 && this.entries.size === 0) {
			return Object.freeze([]);
		}

		const staged: DecorationEntry<TMetadata>[] = [];
		try {
			for (const spec of specs) staged.push(this.createEntry(spec));
		} catch (error) {
			for (const entry of staged) entry.trackedRange.dispose();
			throw error;
		}

		for (const entry of this.entries.values()) {
			entry.trackedRange.dispose();
		}
		this.entries.clear();
		for (const entry of staged) this.entries.set(entry.id, entry);
		this.emitChange(TextDecorationChangeReason.Content);
		return Object.freeze(staged.map(entry => entry.id));
	}

	clear(): void {
		this.replaceAll([]);
	}

	private createEntry(
		spec: TextDecorationSpec<TMetadata>,
	): DecorationEntry<TMetadata> {
		const id = nextTextDecorationId as TextDecorationId;
		nextTextDecorationId += 1;
		return {
			id,
			trackedRange: this.model.trackRange(
				spec.range,
				spec.stickiness,
			),
			metadata: spec.metadata,
			lastRange: spec.range,
		};
	}

	private validateSpec(spec: TextDecorationSpec<TMetadata>): void {
		this.model.offsetAt(spec.range.start);
		this.model.offsetAt(spec.range.end);
	}

	private acceptModelChange(): void {
		let changed = false;
		for (const entry of this.entries.values()) {
			const range = entry.trackedRange.range;
			if (!rangesEqual(range, entry.lastRange)) {
				entry.lastRange = range;
				changed = true;
			}
		}
		if (changed) this.emitChange(TextDecorationChangeReason.Range);
	}

	private createSnapshot(): readonly TextDecorationSnapshot<TMetadata>[] {
		return Object.freeze(
			[...this.entries.values()].map(snapshotEntry),
		);
	}

	private emitChange(reason: TextDecorationChangeReason): void {
		this.changeEmitter.fire(Object.freeze({
			reason,
			modelVersion: this.model.version,
			decorations: this.createSnapshot(),
		}));
	}

}

function snapshotEntry<TMetadata>(
	entry: DecorationEntry<TMetadata>,
): TextDecorationSnapshot<TMetadata> {
	return Object.freeze({
		id: entry.id,
		range: entry.lastRange,
		metadata: entry.metadata,
	});
}

function rangesEqual(left: TextRange, right: TextRange): boolean {
	return left.start.compareTo(right.start) === 0 &&
		left.end.compareTo(right.end) === 0;
}

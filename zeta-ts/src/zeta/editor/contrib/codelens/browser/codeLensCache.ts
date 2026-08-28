import { DisposableStore, type IDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { URI } from '../../../../base/common/uri.js';
import { type IStorageService, StorageScope, StorageTarget } from '../../../../platform/storage/common/storage.js';
import { TextPosition, TextRange } from '../../../common/core/text.js';
import { type LanguageCodeLensProvider } from '../common/codelens.js';
import { CodeLensModel, type CodeLensItem } from './codelens.js';

const MaximumEntries = 20;
const TrimmedEntries = 15;
const StorageKey = 'codelens/cache2';

interface SerializedCacheEntry {
	readonly lineCount: number;
	readonly lines: readonly number[];
}

interface CacheEntry {
	readonly lineCount: number;
	readonly model: CodeLensModel;
}

const cachedCodeLensProvider: LanguageCodeLensProvider = {
	languageIds: ['*'],
	provideCodeLenses: () => { throw new Error('Cached CodeLens entries cannot provide results'); },
};

/** Owns the bounded non-executable cache and its optional workspace storage binding. */
class CodeLensCache {
	private readonly entries = new Map<string, CacheEntry>();
	private storageBinding: IDisposable | undefined;

	public put(resource: URI, lineCount: number, model: CodeLensModel): void {
		const key = resource.toString();
		const lenses = model.lenses.map(item => createCachedItem(item.symbol.range, item.symbol.command?.title));
		this.set(key, { lineCount, model: new CodeLensModel(lenses) });
	}

	public get(resource: URI, lineCount: number): CodeLensModel | undefined {
		const key = resource.toString();
		const entry = this.entries.get(key);
		if (!entry || entry.lineCount !== lineCount) return undefined;
		this.entries.delete(key);
		this.entries.set(key, entry);
		return entry.model;
	}

	public delete(resource: URI): void {
		this.entries.delete(resource.toString());
	}

	public bindStorage(storageService: IStorageService): IDisposable {
		if (this.storageBinding) throw new Error('CodeLens cache storage is already bound');
		this.restore(storageService.get(StorageKey, StorageScope.WORKSPACE, '{}'));
		const binding = new DisposableStore();
		this.storageBinding = binding;
		binding.add(storageService.onWillSaveState(() => this.store(storageService)));
		binding.add(storageService.onDidChangeValue(event => {
			if (event.external && event.scope === StorageScope.WORKSPACE && event.key === StorageKey) {
				this.restore(storageService.get(StorageKey, StorageScope.WORKSPACE, '{}'));
			}
		}));
		binding.add(toDisposable(() => {
			this.store(storageService);
			if (this.storageBinding === binding) this.storageBinding = undefined;
		}));
		return binding;
	}

	private set(key: string, entry: CacheEntry): void {
		this.entries.delete(key);
		this.entries.set(key, entry);
		if (this.entries.size <= MaximumEntries) return;
		while (this.entries.size > TrimmedEntries) {
			const oldestKey = this.entries.keys().next().value as string | undefined;
			if (oldestKey === undefined) return;
			this.entries.delete(oldestKey);
		}
	}

	private store(storageService: IStorageService): void {
		const serialized: Record<string, SerializedCacheEntry> = Object.create(null);
		for (const [key, entry] of this.entries) {
			const lines = new Set(entry.model.lenses.map(item => item.symbol.range.start.lineIndex));
			serialized[key] = { lineCount: entry.lineCount, lines: [...lines].sort((left, right) => left - right) };
		}
		storageService.store(StorageKey, JSON.stringify(serialized), StorageScope.WORKSPACE, StorageTarget.MACHINE);
	}

	private restore(raw: string): void {
		const restored = deserialize(raw);
		this.entries.clear();
		for (const [key, entry] of restored) this.set(key, entry);
	}
}

export const codeLensCache = new CodeLensCache();

export function bindCodeLensCacheStorage(storageService: IStorageService): IDisposable {
	return codeLensCache.bindStorage(storageService);
}

function createCachedItem(range: TextRange, title: string | undefined): CodeLensItem {
	return Object.freeze({
		symbol: Object.freeze({
			range,
			...(title ? { command: Object.freeze({ id: '', title }) } : {}),
		}),
		provider: cachedCodeLensProvider,
	});
}

function deserialize(raw: string): ReadonlyMap<string, CacheEntry> {
	const result = new Map<string, CacheEntry>();
	let value: unknown;
	try {
		value = JSON.parse(raw);
	} catch {
		return result;
	}
	if (!isRecord(value)) return result;
	for (const [key, candidate] of Object.entries(value)) {
		if (!isSerializedCacheEntry(candidate)) continue;
		try {
			URI.parse(key);
		} catch {
			continue;
		}
		const lines = [...new Set(candidate.lines)].filter(line => line < candidate.lineCount).sort((left, right) => left - right);
		const lenses = lines.map(line => createCachedItem(TextRange.emptyAt(TextPosition.at(line, 0)), undefined));
		result.set(key, { lineCount: candidate.lineCount, model: new CodeLensModel(lenses) });
	}
	return result;
}

function isSerializedCacheEntry(value: unknown): value is SerializedCacheEntry {
	if (!isRecord(value) || !Number.isSafeInteger(value.lineCount) || (value.lineCount as number) < 1 || !Array.isArray(value.lines)) return false;
	return value.lines.every(line => Number.isSafeInteger(line) && line >= 0);
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

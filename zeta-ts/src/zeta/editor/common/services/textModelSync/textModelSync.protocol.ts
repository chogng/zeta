import type { TextSnapshot } from '../../core/text.js';

export interface LanguageWorkerDocumentChange {
	readonly rangeOffset: number;
	readonly rangeLength: number;
	readonly text: string;
}

export interface LanguageWorkerDocumentSynchronization {
	readonly previousVersion: number;
	readonly modelVersion: number;
	readonly changes: readonly LanguageWorkerDocumentChange[];
	readonly snapshot: TextSnapshot;
}

export interface LanguageWorkerDocumentSynchronizationObserver {
	synchronizeDocument(synchronization: LanguageWorkerDocumentSynchronization): void;
}

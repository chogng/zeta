import type { SerializedEditorInput } from './editorInputSerializer.js';

export interface EditorWorkingSetEntry {
	readonly input: SerializedEditorInput;
	readonly preview: boolean;
}

export interface EditorGroupWorkingSet {
	readonly editors: readonly EditorWorkingSetEntry[];
	readonly activeEditorIndex: number;
	readonly size: number;
}

/** Serializable editor groups, tabs, preview state, selection, and layout. */
export interface EditorWorkingSet {
	readonly id: string;
	readonly activeGroupIndex: number;
	readonly groups: readonly EditorGroupWorkingSet[];
}

export type EditorWorkingSetTarget = EditorWorkingSet | 'empty';

export interface ApplyEditorWorkingSetOptions {
	readonly preserveFocus?: boolean;
}

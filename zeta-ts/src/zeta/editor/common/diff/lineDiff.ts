export enum LineDiffKind {
	Unchanged = "unchanged",
	Modified = "modified",
	Removed = "removed",
	Added = "added",
}

export interface DiffRange {
	readonly startColumn: number;
	readonly endColumn: number;
}

/** One aligned visual row in a side-by-side line diff. */
export interface LineDiffRow {
	readonly kind: LineDiffKind;
	readonly originalLineIndex?: number;
	readonly modifiedLineIndex?: number;
	readonly originalChanges: readonly DiffRange[];
	readonly modifiedChanges: readonly DiffRange[];
}

/** One changed hunk in the aligned row projection. */
export interface LineDiffHunk {
	readonly rowStart: number;
	readonly rowEnd: number;
	readonly originalStartLineIndex: number;
	readonly originalLineCount: number;
	readonly modifiedStartLineIndex: number;
	readonly modifiedLineCount: number;
}

export interface LineDiff {
	readonly rows: readonly LineDiffRow[];
	readonly hunks: readonly LineDiffHunk[];
}

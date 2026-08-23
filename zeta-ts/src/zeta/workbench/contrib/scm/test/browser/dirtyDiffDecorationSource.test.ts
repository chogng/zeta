import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../../base/common/event.js";
import { URI } from "../../../../../base/common/uri.js";
import { DecorationPresentation } from "../../../../../editor/browser/viewparts/decorations/decorationPresentation.js";
import { TextModel } from "../../../../../editor/common/model/textModel.js";
import { type IDiffApi } from "../../../../../platform/diff/common/diffApi.js";
import { type GitStatus, type IGitService } from "../../../../services/git/common/gitService.js";
import { DirtyDiffDecorationSource } from "../../browser/dirtyDiffDecorationSource.js";

test("Dirty Diff projects Git index changes onto live editor lines", async () => {
	const statusChanged = new Emitter<GitStatus>();
	const becameReady = new Emitter<void>();
	const requests: Array<{ readonly path: string; readonly comparison: string }> = [];
	const status: GitStatus = {
		streamInstanceId: "git-1",
		revision: 7,
		workspacePath: "/workspace",
		head: { type: "branch", name: "main", objectId: "abc", upstream: undefined },
		changes: [{
			path: "src/file.ts",
			originalPath: undefined,
			indexStatus: "unmodified",
			worktreeStatus: "modified",
			conflicted: false,
			submodule: { isSubmodule: false, commitChanged: false, trackedChanges: false, untrackedChanges: false },
		}],
	};
	const gitService = {
		onDidChangeStatus: statusChanged.event,
		onDidBecomeReady: becameReady.event,
		status: async () => status,
		changeFile: async (path: string, comparison: string) => {
			requests.push({ path, comparison });
			return {
				original: { kind: "text" as const, text: "same\nold\nremoved\nlast" },
				modified: { kind: "text" as const, text: "same\nnew\nlast" },
			};
		},
	} as unknown as IGitService;
	const diffApi: IDiffApi = {
		compute: async () => ({
			rows: [
				{ kind: "context", originalLineIndex: 0, modifiedLineIndex: 0, originalChanges: [], modifiedChanges: [] },
				{ kind: "modified", originalLineIndex: 1, modifiedLineIndex: 1, originalChanges: [], modifiedChanges: [] },
				{ kind: "removed", originalLineIndex: 2, modifiedLineIndex: null, originalChanges: [], modifiedChanges: [] },
				{ kind: "context", originalLineIndex: 3, modifiedLineIndex: 2, originalChanges: [], modifiedChanges: [] },
			],
			hunks: [],
			originalLineCount: 4,
			modifiedLineCount: 3,
		}),
	};
	using model = new TextModel("same\nnew\nlast");
	using source = new DirtyDiffDecorationSource(URI.file("/workspace/src/file.ts"), model, gitService, diffApi);

	await source.refresh();

	assert.deepEqual(requests, [{ path: "src/file.ts", comparison: "unstaged" }]);
	assert.deepEqual(source.decorations.map(decoration => [decoration.presentation, decoration.range.start.lineIndex]), [
		[DecorationPresentation.DiffModified, 1],
		[DecorationPresentation.DiffDeleted, 2],
	]);
	statusChanged.dispose();
	becameReady.dispose();
});

import { URI } from '../../../../base/common/uri.js';
import type { EditorInput } from '../../../services/editor/common/editorService.js';
import type { GitChangeFileComparison, GitCommitFileContent, GitRepositoryChange, GitStatus, IGitService } from '../../../services/git/common/gitService.js';

export interface ResolvedGitChangeInputs {
	readonly original: EditorInput | undefined;
	readonly modified: EditorInput | undefined;
}

/** Resolves one Git change into the ordinary text-resource inputs used by diff panes. */
export async function resolveGitChangeInputs(gitService: IGitService, status: GitStatus, change: GitRepositoryChange, comparison: GitChangeFileComparison): Promise<ResolvedGitChangeInputs> {
	const file = await gitService.changeFile(change.path, comparison);
	const originalPath = changeOriginalPath(change, comparison);
	const [originalState, modifiedState] = comparison === 'staged'
		? ['HEAD', 'Index'] as const
		: ['Index', 'Working Tree'] as const;
	return {
		original: changeEditorInput(file.original, changeFileUri(status, comparison, originalPath, 'original'), `${basename(originalPath)} (${originalState})`),
		modified: changeEditorInput(file.modified, changeFileUri(status, comparison, change.path, 'modified'), `${basename(change.path)} (${modifiedState})`),
	};
}

function changeOriginalPath(change: GitRepositoryChange, comparison: GitChangeFileComparison): string {
	const status = comparison === 'staged' ? change.indexStatus : change.worktreeStatus;
	return status === 'renamed' || status === 'copied' ? change.originalPath ?? change.path : change.path;
}

function changeEditorInput(content: GitCommitFileContent, resource: URI, label: string): EditorInput | undefined {
	if (content.kind === 'binary') return undefined;
	return {
		resource,
		label,
		readOnly: true,
		initialText: content.kind === 'missing' ? '' : content.text,
	};
}

function changeFileUri(status: GitStatus, comparison: GitChangeFileComparison, path: string, side: 'original' | 'modified'): URI {
	const encodedPath = path.split('/').map(encodeURIComponent).join('/');
	const query = new URLSearchParams({
		side,
		stream: status.streamInstanceId,
		revision: String(status.revision),
	});
	return URI.parse(`git-change:/${comparison}/${encodedPath}?${query}`);
}

function basename(path: string): string {
	return path.replaceAll('\\', '/').split('/').at(-1) ?? path;
}

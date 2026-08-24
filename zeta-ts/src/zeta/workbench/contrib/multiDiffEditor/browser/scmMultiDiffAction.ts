import { URI } from '../../../../base/common/uri.js';
import { Action2 } from '../../../../platform/actions/common/actions.js';
import type { ServicesAccessor } from '../../../../platform/instantiation/common/instantiation.js';
import { IEditorService } from '../../../services/editor/common/editorService.js';
import type { GitChangeFileComparison, GitRepositoryChange, GitStatus } from '../../../services/git/common/gitService.js';
import { IGitService } from '../../../services/git/common/gitService.js';
import { resolveGitChangeInputs } from '../../scm/browser/scmChangeEditorInput.js';
import { createMultiDiffEditorInput, type MultiDiffEditorInputItem } from './multiDiffEditorInput.js';

export const OpenScmMultiDiffEditorCommandId = '_workbench.openScmMultiDiffEditor';

export interface OpenScmMultiDiffEditorOptions {
	readonly title: string;
	readonly comparison: GitChangeFileComparison;
	readonly status: GitStatus;
	readonly changes: readonly GitRepositoryChange[];
}

export type OpenScmMultiDiffEditorResult = 'opened' | 'empty' | 'stale';

/** Internal SCM command that resolves one resource group into a MultiDiff input. */
export class OpenScmMultiDiffEditorAction extends Action2 {
	constructor() {
		super({
			id: OpenScmMultiDiffEditorCommandId,
			title: 'Open Changes',
			f1: false,
		});
	}

	public override async run(accessor: ServicesAccessor, rawOptions: unknown): Promise<OpenScmMultiDiffEditorResult> {
		const options = validateOptions(rawOptions);
		const gitService = accessor.get(IGitService);
		const items: MultiDiffEditorInputItem[] = [];
		for (const change of options.changes) {
			if (change.conflicted) continue;
			const inputs = await resolveGitChangeInputs(gitService, options.status, change, options.comparison);
			if (inputs.original && inputs.modified) {
				items.push({
					label: change.originalPath ? `${change.originalPath} → ${change.path}` : change.path,
					original: inputs.original,
					modified: inputs.modified,
				});
			}
		}
		if (items.length === 0) return 'empty';
		const currentStatus = await gitService.status();
		if (!isSameStatus(currentStatus, options.status)) return 'stale';
		const section = options.comparison === 'staged' ? 'index' : 'worktree';
		const source = URI.parse(`zeta-multi-diff:/scm/${section}?stream=${encodeURIComponent(options.status.streamInstanceId)}&revision=${options.status.revision}`);
		await accessor.get(IEditorService).openEditor(createMultiDiffEditorInput(source, items, options.title), { pinned: true });
		return 'opened';
	}
}

function validateOptions(value: unknown): OpenScmMultiDiffEditorOptions {
	if (!value || typeof value !== 'object') throw new TypeError('Open SCM multi-diff requires options');
	const options = value as Partial<OpenScmMultiDiffEditorOptions>;
	if (typeof options.title !== 'string' || options.title.trim().length === 0) throw new TypeError('Open SCM multi-diff requires a title');
	if (options.comparison !== 'staged' && options.comparison !== 'unstaged') throw new TypeError('Open SCM multi-diff requires a comparison');
	if (!options.status || typeof options.status !== 'object' || !Array.isArray(options.changes)) throw new TypeError('Open SCM multi-diff requires a Git status and changes');
	return options as OpenScmMultiDiffEditorOptions;
}

function isSameStatus(first: GitStatus, second: GitStatus): boolean {
	return first.streamInstanceId === second.streamInstanceId && first.revision === second.revision;
}

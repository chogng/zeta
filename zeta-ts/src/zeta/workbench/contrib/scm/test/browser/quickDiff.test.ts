import assert from 'node:assert/strict';
import test from 'node:test';
import { Emitter } from '../../../../../base/common/event.js';
import { URI } from '../../../../../base/common/uri.js';
import { DecorationPresentation } from '../../../../../editor/browser/viewparts/decorations/decorations.js';
import { TextModel } from '../../../../../editor/common/model/textModel.js';
import { type IConfigurationService } from '../../../../../platform/configuration/common/configurationService.js';
import { type IDiffApi } from '../../../../../platform/diff/common/diffApi.js';
import { AppServerDiffService } from '../../../../services/diff/browser/appServerDiffService.js';
import { type GitStatus, type IGitService } from '../../../../services/git/common/gitService.js';
import { GitQuickDiffProvider } from '../../browser/gitQuickDiffProvider.js';
import { QuickDiffDecorator } from '../../browser/quickDiffDecorator.js';
import { QuickDiffModelService } from '../../browser/quickDiffModel.js';
import { WorkbenchQuickDiffService } from '../../browser/workbenchQuickDiffService.js';

test('Git Quick Diff supplies the index for a live worktree change', async () => {
	const fixture = gitFixture();
	using provider = new GitQuickDiffProvider(fixture.gitService);

	const original = await provider.provideOriginalResource(URI.file('/workspace/src/file.ts'), new AbortController().signal);

	assert.equal(original?.label, 'Index');
	assert.equal(original?.text, 'same\nold\nremoved\nlast');
	assert.deepEqual(fixture.requests, [{ path: 'src/file.ts', comparison: 'unstaged' }]);
	fixture.dispose();
});

test('Quick Diff shares one resource model and projects configurable editor targets', async () => {
	const fixture = gitFixture();
	using provider = new GitQuickDiffProvider(fixture.gitService);
	using quickDiffService = new WorkbenchQuickDiffService();
	using providerRegistration = quickDiffService.addProvider(provider);
	using modelService = new QuickDiffModelService(quickDiffService, new AppServerDiffService(fixture.diffApi));
	using model = new TextModel('same\nnew\nlast');
	const firstReference = modelService.createModelReference(URI.file('/workspace/src/file.ts'), model);
	const secondReference = modelService.createModelReference(URI.file('/workspace/src/file.ts'), model);
	assert.equal(firstReference.object, secondReference.object);
	firstReference.dispose();

	using source = new QuickDiffDecorator(
		URI.file('/workspace/src/file.ts'),
		model,
		modelService,
		configurationService('all'),
	);
	await waitFor(() => source.decorations.length === 2);

	assert.deepEqual(source.decorations.map(decoration => [decoration.presentation, decoration.range.start.lineIndex]), [
		[DecorationPresentation.DiffModified, 1],
		[DecorationPresentation.DiffDeleted, 2],
	]);
	assert.ok(source.decorations.every(decoration => decoration.linesDecoration?.className?.includes('zeta-quick-diff-gutter')));
	assert.ok(source.decorations.every(decoration => decoration.overviewRuler === true && decoration.minimap === true));
	assert.ok(secondReference.object.findChangeAtLine(2), 'the deletion gutter line resolves to its containing hunk');

	secondReference.dispose();
	fixture.dispose();
});

function gitFixture(): { readonly gitService: IGitService; readonly diffApi: IDiffApi; readonly requests: Array<{ readonly path: string; readonly comparison: string }>; dispose(): void } {
	const statusChanged = new Emitter<GitStatus>();
	const repositoriesChanged = new Emitter<never>();
	const becameReady = new Emitter<void>();
	const requests: Array<{ readonly path: string; readonly comparison: string }> = [];
	const status: GitStatus = {
		repositoryId: 'repo-1',
		streamInstanceId: 'git-1',
		revision: 7,
		workspacePath: '/workspace',
		head: { type: 'branch', name: 'main', objectId: 'abc', upstream: undefined },
		changes: [{
			path: 'src/file.ts',
			originalPath: undefined,
			indexStatus: 'unmodified',
			worktreeStatus: 'modified',
			conflicted: false,
			submodule: { isSubmodule: false, commitChanged: false, trackedChanges: false, untrackedChanges: false },
		}],
	};
	const gitService = {
		onDidChangeStatus: statusChanged.event,
		onDidChangeRepositoryStatus: statusChanged.event,
		onDidChangeRepositories: repositoriesChanged.event,
		onDidBecomeReady: becameReady.event,
		repositoryForResource: () => ({ id: status.repositoryId, label: 'workspace', path: '', root: URI.file('/workspace') }),
		listRepositories: async () => [],
		status: async () => status,
		changeFile: async (path: string, comparison: string) => {
			requests.push({ path, comparison });
			return {
				original: { kind: 'text' as const, text: 'same\nold\nremoved\nlast' },
				modified: { kind: 'text' as const, text: 'same\nnew\nlast' },
			};
		},
	} as unknown as IGitService;
	const diffApi: IDiffApi = {
		compute: async () => ({
			rows: [
				{ kind: 'context', originalLineIndex: 0, modifiedLineIndex: 0, originalChanges: [], modifiedChanges: [] },
				{ kind: 'modified', originalLineIndex: 1, modifiedLineIndex: 1, originalChanges: [], modifiedChanges: [] },
				{ kind: 'removed', originalLineIndex: 2, modifiedLineIndex: null, originalChanges: [], modifiedChanges: [] },
				{ kind: 'context', originalLineIndex: 3, modifiedLineIndex: 2, originalChanges: [], modifiedChanges: [] },
			],
			hunks: [],
			originalLineCount: 4,
			modifiedLineCount: 3,
		}),
	};
	return {
		gitService,
		diffApi,
		requests,
		dispose(): void {
			statusChanged.dispose();
			repositoriesChanged.dispose();
			becameReady.dispose();
		},
	};
}

function configurationService(value: 'all'): IConfigurationService {
	return {
		onDidChangeConfiguration: () => ({ dispose(): void {}, [Symbol.dispose](): void {} }),
		getValue: () => value,
		updateValue: async () => undefined,
		resetValue: async () => undefined,
		reload: async () => undefined,
	} as IConfigurationService;
}

async function waitFor(condition: () => boolean, timeoutMillis = 1_000): Promise<void> {
	const deadline = Date.now() + timeoutMillis;
	while (!condition()) {
		if (Date.now() >= deadline) throw new Error('Timed out waiting for Quick Diff');
		await new Promise(resolve => setTimeout(resolve, 0));
	}
}

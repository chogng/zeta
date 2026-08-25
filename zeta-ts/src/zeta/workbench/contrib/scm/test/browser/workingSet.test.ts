import assert from 'node:assert/strict';
import test from 'node:test';
import { Emitter } from '../../../../../base/common/event.js';
import type { IConfigurationChangeEvent, IConfigurationKey, IConfigurationService } from '../../../../../platform/configuration/common/configurationService.js';
import { StorageScope, type IStorageService } from '../../../../../platform/storage/common/storage.js';
import type { IEditorPart } from '../../../../browser/parts/editor/editorPart.js';
import type { EditorWorkingSet, EditorWorkingSetTarget } from '../../../../services/editor/common/editorWorkingSet.js';
import type { GitStatus, IGitService } from '../../../../services/git/common/gitService.js';
import { ScmWorkingSetController } from '../../browser/workingSet.js';
import { ScmConfiguration } from '../../common/scmConfiguration.js';

test('SCM working sets save and restore editor state across branch changes', async () => {
	using configuration = new TestConfigurationService();
	configuration.values.set(ScmConfiguration.workingSetsEnabled.key, true);
	configuration.values.set(ScmConfiguration.workingSetsDefault.key, 'empty');
	using git = new TestGitService(status('main', 1));
	const storage = new TestStorageService();
	const saved: string[] = [];
	const applied: EditorWorkingSetTarget[] = [];
	const editorPart = {
		domNode: globalThis.document?.body ?? ({ ownerDocument: { activeElement: undefined }, contains: () => false } as unknown as HTMLElement),
		saveWorkingSet(id: string): EditorWorkingSet {
			saved.push(id);
			return workingSet(id);
		},
		async applyWorkingSet(target: EditorWorkingSetTarget): Promise<void> {
			applied.push(target);
		},
	} as unknown as IEditorPart;
	using controller = new ScmWorkingSetController({
		configurationService: configuration as unknown as IConfigurationService,
		editorPart,
		gitService: git as unknown as IGitService,
		storageService: storage as unknown as IStorageService,
	});
	await nextTask();

	git.accept(status('feature', 2));
	await nextTask();
	assert.deepEqual(saved, ['main']);
	assert.deepEqual(applied, ['empty']);
	assert.ok(storage.get('scm.workingSets', StorageScope.WORKSPACE)?.includes('main'));

	git.accept(status('main', 3));
	await nextTask();
	assert.deepEqual(saved, ['main', 'feature']);
	assert.deepEqual(applied, ['empty', workingSet('main')]);

	configuration.set(ScmConfiguration.workingSetsEnabled, false);
	assert.equal(storage.get('scm.workingSets', StorageScope.WORKSPACE), undefined);
	git.accept(status('other', 4));
	await nextTask();
	assert.deepEqual(saved, ['main', 'feature']);
});

function workingSet(id: string): EditorWorkingSet {
	return {
		id,
		activeGroupIndex: 0,
		groups: [{ activeEditorIndex: -1, editors: [], size: 1 }],
	};
}

function status(branch: string, revision: number): GitStatus {
	return {
		repositoryId: 'repo-1',
		streamInstanceId: 'git_test',
		revision,
		workspacePath: 'C:/project',
		head: { type: 'branch', name: branch, objectId: branch.padEnd(40, '0'), upstream: undefined },
		changes: [],
	};
}

class TestGitService implements Disposable {
	private readonly statusChanged = new Emitter<GitStatus>();
	private readonly becameReady = new Emitter<void>();
	readonly onDidChangeStatus = this.statusChanged.event;
	readonly onDidBecomeReady = this.becameReady.event;

	constructor(private current: GitStatus) {}

	status(): Promise<GitStatus> {
		return Promise.resolve(this.current);
	}

	accept(value: GitStatus): void {
		this.current = value;
		this.statusChanged.fire(value);
	}

	dispose(): void {
		this.statusChanged.dispose();
		this.becameReady.dispose();
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

class TestConfigurationService implements Disposable {
	readonly values = new Map<string, unknown>();
	private readonly changed = new Emitter<IConfigurationChangeEvent>();
	readonly onDidChangeConfiguration = this.changed.event;

	getValue<T>(key: IConfigurationKey<T>): T {
		return (this.values.get(key.key) ?? key.defaultValue) as T;
	}

	set<T>(key: IConfigurationKey<T>, value: T): void {
		this.values.set(key.key, value);
		this.changed.fire({ keys: new Set([key.key]), affectsConfiguration: candidate => candidate.key === key.key });
	}

	dispose(): void {
		this.changed.dispose();
	}

	[Symbol.dispose](): void {
		this.dispose();
	}
}

class TestStorageService {
	private readonly values = new Map<string, string>();

	get(key: string, _scope: StorageScope): string | undefined {
		return this.values.get(key);
	}

	store(key: string, value: string): void {
		this.values.set(key, value);
	}

	remove(key: string): void {
		this.values.delete(key);
	}
}

function nextTask(): Promise<void> {
	return new Promise(resolve => globalThis.setTimeout(resolve, 0));
}

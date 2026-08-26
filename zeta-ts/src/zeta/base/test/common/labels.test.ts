import assert from 'node:assert/strict';
import test from 'node:test';
import { getPathLabel, shorten, splitRecentLabel, template, tildify, type IRelativePathProvider } from '../../common/labels.js';
import { OperatingSystem } from '../../common/platform.js';
import { URI } from '../../common/uri.js';

test('getPathLabel uses target separators, home shortening, and workspace-relative labels', () => {
	const root = URI.file('/workspace');
	const resource = URI.file('/workspace/src/index.ts');
	const relative: IRelativePathProvider = {
		getWorkspace: () => ({ folders: [{ uri: root, name: 'project' }] }),
		getWorkspaceFolder: candidate => candidate.toString().startsWith(root.toString()) ? { uri: root, name: 'project' } : null,
	};

	assert.equal(getPathLabel(resource, { os: OperatingSystem.Linux, relative }), 'src/index.ts');
	assert.equal(getPathLabel(resource, { os: OperatingSystem.Windows }), '\\workspace\\src\\index.ts');
	assert.equal(getPathLabel(URI.file('/home/zeta/file.ts'), {
		os: OperatingSystem.Linux,
		tildify: { userHome: URI.file('/home/zeta') },
	}), '~/file.ts');
});

test('tildify is case-sensitive on Linux and case-insensitive on macOS', () => {
	assert.equal(tildify('/Users/Zeta/project', '/users/zeta', OperatingSystem.Linux), '/Users/Zeta/project');
	assert.equal(tildify('/Users/Zeta/project', '/users/zeta', OperatingSystem.Macintosh), '~/project');
	assert.equal(tildify('/users/zeta', '/users/zeta', OperatingSystem.Linux), '/users/zeta');
});

test('shorten retains root context and distinguishes common suffixes', () => {
	assert.deepEqual(shorten(['a/b', 'a/c'], '/'), ['…/b', '…/c']);
	assert.deepEqual(shorten(['/a/b', '/a/c'], '/'), ['/a/b', '/a/c']);
	assert.deepEqual(shorten(['a/b/c', 'd/b/c'], '/'), ['a/…', 'd/…']);
	assert.deepEqual(shorten(['', 'a'], '/'), ['./', 'a']);
});

test('template omits conditional separators beside empty values', () => {
	assert.equal(template('${left}${separator}${right}', { left: '', right: 'zeta', separator: { label: ' — ' } }), 'zeta');
	assert.equal(template('${left}${separator}${right}', { left: 'Zeta', right: 'Code', separator: { label: ' — ' } }), 'Zeta — Code');
});

test('mnemonic and recent labels retain VS Code-compatible semantics', () => {
	assert.equal(template('Foo${missing}Bar'), 'FooBar');
	assert.equal(splitRecentLabel('/workspace/project [SSH: host]').name, 'project [SSH: host]');
	assert.equal(splitRecentLabel('/workspace/project [SSH: host]').parentPath, '/workspace');
});

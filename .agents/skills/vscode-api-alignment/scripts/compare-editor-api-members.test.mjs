import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { readDeclaration } from './compare-editor-api-members.mjs';

test('includes inherited public Editor members without importing external base classes', () => {
	const directory = mkdtempSync(join(tmpdir(), 'zeta-editor-api-members-'));
	try {
		const base = join(directory, 'base.ts');
		const child = join(directory, 'child.ts');
		writeFileSync(base, [
			'export class Base {',
			'  public inherited(): void {}',
			'  protected projected(): void {}',
			'  private hidden(): void {}',
			'}',
		].join('\n'));
		writeFileSync(child, [
			"import { Base } from './base.js';",
			'export class Child extends Base {',
			'  own(): void {}',
			'}',
		].join('\n'));

		const declaration = readDeclaration(child, 'Child', directory);
		assert.deepEqual(declaration?.declaredMembers, ['own']);
		assert.deepEqual(declaration?.members, [
			'inherited',
			'own',
			'projected',
		]);
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
});

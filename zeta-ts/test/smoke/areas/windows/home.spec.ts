import { access, realpath } from 'node:fs/promises';
import { join } from 'node:path';
import { expect, test } from '../../../automation/test.js';

test('Desktop uses the selected Zeta home for UI and backend startup', async ({ application, target, workbench }) => {
	test.skip(target.kind !== 'electron', 'This scenario verifies process startup on the desktop host.');
	if (target.kind !== 'electron' || !('windows' in application)) {
		return;
	}
	const paths = await application.evaluate(({ app }) => ({
		home: process.env.ZETA_HOME,
		legacy: process.env.ZETA_PROFILE_ROOT,
		userData: app.getPath('userData'),
	}));
	expect(paths.legacy).toBeUndefined();
	expect(paths.home).toBeDefined();
	expect(await realpath(paths.home!)).toBe(await realpath(join(paths.userData, 'profile')));
	await expect(workbench.element).toBeVisible();
	if (target.appServerMode === 'required') {
		await expect.poll(async () => access(join(paths.home!, 'state.sqlite3')).then(() => true, () => false)).toBe(true);
	}
});

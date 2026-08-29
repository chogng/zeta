import assert from "node:assert/strict";
import {
	mkdtemp,
	readFile,
	rm,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { URI } from "../../../../base/common/uri.js";
import {
	ConfigurationRegistry,
} from "../../../../platform/configuration/common/configurationRegistry.js";
import {
	type IConfigurationApi,
	type IConfigurationSnapshot,
	type IConfigurationUpdateRequest,
	configurationValues,
	validateConfigurationDocument,
	validateConfigurationSnapshot,
	validateConfigurationUpdateRequest,
} from "../../../../platform/configuration/common/configurationIpc.js";
import {
	ConfigurationMainService,
} from "../../../../platform/configuration/electron-main/configurationMainService.js";
import {
	WorkbenchConfigurationService,
} from "../../../../workbench/services/configuration/browser/configurationService.js";
import {
	ConfigurationResourceRevisionConflictError,
} from "../../../../platform/configuration/common/configurationResourceService.js";
import { ConfigurationTarget, getConfigValueInTarget, isConfigurationOverrides } from "../../../../platform/configuration/common/configurationService.js";

test("configuration contracts expose target inspection and validate overrides", () => {
	const values = { defaultValue: 12, userValue: 14, workspaceValue: 16 };
	assert.equal(getConfigValueInTarget(values, ConfigurationTarget.DEFAULT), 12);
	assert.equal(getConfigValueInTarget(values, ConfigurationTarget.USER), 14);
	assert.equal(getConfigValueInTarget(values, ConfigurationTarget.WORKSPACE), 16);
	assert.equal(getConfigValueInTarget(values, ConfigurationTarget.MEMORY), undefined);
	assert.equal(isConfigurationOverrides({ overrideIdentifier: "typescript" }), true);
	assert.equal(isConfigurationOverrides({ resource: URI.parse("file:///workspace/file.ts") }), true);
	assert.equal(isConfigurationOverrides({ overrideIdentifier: 1 }), false);
	assert.equal(isConfigurationOverrides({ resource: {} }), false);
});

test("configuration validators bound the complete wire document", () => {
	assert.deepEqual(
		validateConfigurationSnapshot({
			revision: 2,
			document: {
				version: 1,
				source: '{ "editor.fontSize": 14 }\n',
			},
		}),
		{
			revision: 2,
			document: {
				version: 1,
				source: '{ "editor.fontSize": 14 }\n',
			},
		},
	);
	assert.throws(
		() => validateConfigurationDocument({
			version: 1,
			source: '{ "__proto__.unsafe": true }',
		}),
		/invalid configuration key/,
	);
	assert.throws(
		() => validateConfigurationDocument({ version: 1, values: { 'editor.fontSize': 14 } }),
		/configuration object must contain exactly: source, version/,
	);
	assert.throws(
		() => validateConfigurationUpdateRequest({
			expectedRevision: -1,
			document: { version: 1, source: '{}' },
		}),
		/non-negative safe integer/,
	);
});

test("workbench configuration resolves typed defaults and live snapshots", async () => {
	const registry = new ConfigurationRegistry();
	const fontSize = registry.registerConfiguration({
		key: "editor.fontSize",
		defaultValue: 12,
		parse(value): number {
			if (!Number.isInteger(value) || (value as number) < 8) {
				throw new TypeError("font size must be an integer of at least 8");
			}
			return value as number;
		},
	});
	const api = new TestConfigurationApi({
		revision: 0,
			document: {
				version: 1,
				source: '{ "editor.fontSize": 16 }\n',
		},
	});
	using service = new WorkbenchConfigurationService({
		api,
		registry,
	});
	let changes = 0;
	using listener = service.onDidChangeConfiguration((event) => {
		if (event.affectsConfiguration(fontSize)) changes += 1;
	});

	assert.equal(service.getValue(fontSize), 12);
	await service.reload();
	assert.equal(service.getValue(fontSize), 16);

	await service.updateValue(fontSize, 18);
	assert.equal(service.getValue(fontSize), 18);

	api.emit({
		revision: 2,
		document: {
			version: 1,
			source: '{ "editor.fontSize": 20 }\n',
		},
	});
	assert.equal(service.getValue(fontSize), 20);
	await service.resetValue(fontSize);
	assert.equal(service.getValue(fontSize), 12);
	assert.equal(changes, 4);
	await assert.rejects(
		() => service.updateValue(fontSize, 4),
		/font size/,
	);
});

test("workbench configuration preserves editable JSONC source", async () => {
	const registry = new ConfigurationRegistry();
	const fontSize = registry.registerConfiguration({
		key: "editor.fontSize",
		defaultValue: 12,
		parse(value): number {
			if (!Number.isInteger(value) || (value as number) < 8) throw new TypeError("font size must be at least 8");
			return value as number;
		},
	});
	using service = new WorkbenchConfigurationService({ registry });
	const observed: string[] = [];
	using listener = service.onDidChangeResource(snapshot => observed.push(snapshot.source));

	assert.deepEqual(await service.read(), { source: "{}\n", revision: 0 });
	const source = `{
		// Keep this user explanation.
		"editor.fontSize": 16,
		"extension.unregistered": true,
	}\n`;
	assert.deepEqual(await service.write(source, 0), {
		source,
		revision: 1,
	});
	assert.equal(service.getValue(fontSize), 16);
	assert.deepEqual(observed, [source]);
	await service.updateValue(fontSize, 18);
	assert.match((await service.read()).source, /Keep this user explanation/u);
	assert.match((await service.read()).source, /"editor\.fontSize": 18/u);
	await assert.rejects(() => service.write('{ "editor.fontSize": 4 }', 2), /font size/);
	await assert.rejects(() => service.write('{}', 0), ConfigurationResourceRevisionConflictError);
	await assert.rejects(() => service.write('[]', 2), /must be an object/);
});

test("main configuration service persists atomic revisions", async (context) => {
	const directory = await mkdtemp(join(tmpdir(), "zeta-configuration-"));
	context.after(async () => {
		await rm(directory, { recursive: true, force: true });
	});
	const filePath = join(directory, "configuration.json");
	const service = await ConfigurationMainService.create({ filePath });

	const updated = await service.update({
		expectedRevision: 0,
		document: {
			version: 1,
			source: '{\n\t// persisted\n\t"editor.fontSize": 14,\n}\n',
		},
	});
	assert.equal(updated.revision, 1);
	await assert.rejects(
		() => service.update({
				expectedRevision: 0,
				document: { version: 1, source: '{}' },
		}),
		/revision conflict/,
	);
	await service.close();

	assert.deepEqual(
		JSON.parse(await readFile(filePath, "utf8")),
		updated.document,
	);
	assert.deepEqual(configurationValues(updated.document), { "editor.fontSize": 14 });
	const reopened = await ConfigurationMainService.create({ filePath });
	assert.deepEqual(reopened.read(), {
		revision: 0,
		document: updated.document,
	});
	await reopened.close();
});

class TestConfigurationApi implements IConfigurationApi {
	private readonly listeners = new Set<(snapshot: unknown) => void>();
	private snapshot: IConfigurationSnapshot;

	constructor(snapshot: IConfigurationSnapshot) {
		this.snapshot = snapshot;
	}

	read(): Promise<unknown> {
		return Promise.resolve(this.snapshot);
	}

	update(
		request: IConfigurationUpdateRequest,
	): Promise<unknown> {
		if (request.expectedRevision !== this.snapshot.revision) {
			return Promise.reject(new Error("revision conflict"));
		}
		this.snapshot = {
			revision: this.snapshot.revision + 1,
			document: request.document,
		};
		return Promise.resolve(this.snapshot);
	}

	onDidChange(
		listener: (snapshot: unknown) => void,
	): { dispose(): void } {
		this.listeners.add(listener);
		return {
			dispose: () => this.listeners.delete(listener),
		};
	}

	emit(snapshot: IConfigurationSnapshot): void {
		this.snapshot = snapshot;
		for (const listener of this.listeners) listener(snapshot);
	}
}

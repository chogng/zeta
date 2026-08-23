import assert from "node:assert/strict";
import {
	mkdtemp,
	readFile,
	rm,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
	ConfigurationRegistry,
} from "../../../../platform/configuration/common/configurationRegistry.js";
import {
	type IConfigurationApi,
	type IConfigurationSnapshot,
	type IConfigurationUpdateRequest,
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

test("configuration validators bound the complete wire document", () => {
	assert.deepEqual(
		validateConfigurationSnapshot({
			revision: 2,
			document: {
				version: 1,
				values: {
					"editor.fontSize": 14,
				},
			},
		}),
		{
			revision: 2,
			document: {
				version: 1,
				values: {
					"editor.fontSize": 14,
				},
			},
		},
	);
	assert.throws(
		() => validateConfigurationDocument({
			version: 1,
			values: { "__proto__.unsafe": true },
		}),
		/invalid configuration key/,
	);
	assert.throws(
		() => validateConfigurationUpdateRequest({
			expectedRevision: -1,
			document: { version: 1, values: {} },
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
			values: { "editor.fontSize": 16 },
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
			values: { "editor.fontSize": 20 },
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
			values: {
				"editor.fontSize": 14,
			},
		},
	});
	assert.equal(updated.revision, 1);
	await assert.rejects(
		() => service.update({
			expectedRevision: 0,
			document: { version: 1, values: {} },
		}),
		/revision conflict/,
	);
	await service.close();

	assert.deepEqual(
		JSON.parse(await readFile(filePath, "utf8")),
		updated.document,
	);
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

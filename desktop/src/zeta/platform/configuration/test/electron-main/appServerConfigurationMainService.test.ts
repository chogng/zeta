import assert from "node:assert/strict";
import test from "node:test";
import type { ConfigReadResult, ConfigUpdateParams, DesktopProductPreferencesDto } from "../../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../../app-server/electron-main/app-server-supervisor.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import type { IConfigurationSnapshot } from "../../common/configuration.js";
import { AppServerConfigurationMainService } from "../../electron-main/appServerConfigurationMainService.js";

test("Desktop configuration migrates supported legacy values and writes canonical product preferences", async () => {
  const host = new TestConfigHost();
  let legacy: IConfigurationSnapshot = {
    revision: 3,
    document: {
      version: 1 as const,
      values: {
        "workbench.colorTheme": "zeta-light",
        "workbench.sash.size": 7,
        "legacy.unowned": true,
      },
    },
  };
  const service = await AppServerConfigurationMainService.create(
    host as unknown as AppServerSupervisor,
    {
      read: () => legacy,
      update: request => {
        legacy = { revision: legacy.revision + 1, document: request.document };
        return Promise.resolve(legacy);
      },
    },
  );

  assert.equal(host.config.products.desktop.colorTheme, "zeta-light");
  assert.equal(host.config.products.desktop.sashSize, 7);
  assert.deepEqual(service.read().document.values, {
    "workbench.colorTheme": "zeta-light",
    "workbench.sash.size": 7,
  });
  assert.deepEqual(legacy.document.values, { "legacy.unowned": true });

  const current = service.read();
  const updated = await service.update({
    expectedRevision: current.revision,
    document: {
      version: 1,
      values: {
        "workbench.colorTheme": "zeta-dark",
        "workbench.hover.delay": 450,
      },
    },
  });
  assert.equal(updated.document.values["workbench.colorTheme"], "zeta-dark");
  assert.equal(host.config.products.desktop.sashSize, undefined);
  assert.equal(host.config.products.desktop.hoverDelayMilliseconds, 450);
  await assert.rejects(
    service.update({
      expectedRevision: updated.revision,
      document: { version: 1, values: { "unowned.setting": true } },
    }),
    /is not supported by config\.toml/,
  );
  service.dispose();
});

class TestConfigHost {
  config: ConfigReadResult = {
    revision: 0,
    generation: 0,
    preferredModel: null,
    approvalReviewModel: { type: "automatic" },
    products: { desktop: {}, code: {}, zeterm: {} },
    providers: {},
    mcpServers: {},
    skillSources: {},
    pluginRequests: {},
    hooks: {},
    languageServers: {},
    toolSearch: { mode: "lexical", embeddingModel: null, embeddingStatus: { type: "disabled" } },
    semanticCodeIndex: { selection: { type: "disabled" }, automaticContext: "off", activeWorkspaceAuthorized: false },
    execPolicyRules: [],
  };

  request(definition: { method: string }, params: unknown): Promise<unknown> {
    if (definition.method === "config/read") return Promise.resolve(structuredClone(this.config));
    if (definition.method !== "config/update") return Promise.reject(new Error(`unexpected method ${definition.method}`));
    const update = params as ConfigUpdateParams;
    if (update.expectedRevision !== this.config.revision) return Promise.reject(new Error("revision conflict"));
    const desktop = update.products?.desktop;
    if (desktop) this.config.products.desktop = applyDesktopUpdate(this.config.products.desktop, desktop);
    this.config.revision += 1;
    this.config.generation += 1;
    return Promise.resolve({ revision: this.config.revision, generation: this.config.generation, disposition: "updated" });
  }

  onNotification(): { dispose(): void } {
    return toDisposable(() => {});
  }

  onStateChange(): { dispose(): void } {
    return toDisposable(() => {});
  }
}

function applyDesktopUpdate(current: DesktopProductPreferencesDto, update: NonNullable<ConfigUpdateParams["products"]>["desktop"]): DesktopProductPreferencesDto {
  const next = { ...current } as Record<string, unknown>;
  for (const [key, value] of Object.entries(update ?? {})) {
    if (value === null) delete next[key];
    else next[key] = value;
  }
  return next as DesktopProductPreferencesDto;
}

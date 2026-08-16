import { randomUUID } from "node:crypto";
import { APP_SERVER_METHODS, type AutomaticPreferenceDto, type ConfigReadResult, type DesktopProductPreferencesUpdateDto } from "../../../../../generated/app-server/types.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { type IConfigurationDocument, type IConfigurationSnapshot, type IConfigurationUpdateRequest, validateConfigurationDocument } from "../common/configuration.js";

const desktopKeys = {
  colorTheme: "workbench.colorTheme",
  accessibilitySupport: "editor.accessibilitySupport",
  reduceMotion: "workbench.reduceMotion",
  reduceTransparency: "workbench.reduceTransparency",
  underlineLinks: "accessibility.underlineLinks",
  hoverDelayMilliseconds: "workbench.hover.delay",
  reducedHoverDelayMilliseconds: "workbench.hover.reducedDelay",
  sashSize: "workbench.sash.size",
  sashHoverDelayMilliseconds: "workbench.sash.hoverDelay",
} as const;

export interface LegacyConfigurationReader {
  read(): IConfigurationSnapshot;
  update(request: IConfigurationUpdateRequest): Promise<IConfigurationSnapshot>;
}

/** Projects the canonical App Server Config snapshot into Desktop's typed key API. */
export class AppServerConfigurationMainService extends DisposableOwner {
  private readonly _onDidChange = this.own(new Emitter<IConfigurationSnapshot>());
  private snapshot: IConfigurationSnapshot;
  private refreshPromise: Promise<IConfigurationSnapshot> | undefined;

  private constructor(private readonly supervisor: AppServerSupervisor, config: ConfigReadResult) {
    super();
    this.snapshot = desktopSnapshot(config);
    this.own(supervisor.onNotification(notification => {
      if (notification.method === "config/changed") void this.refresh().catch(error => console.error("Failed to refresh Desktop configuration", error));
    }));
    this.own(supervisor.onStateChange(state => {
      if (state === "ready") void this.refresh().catch(error => console.error("Failed to refresh Desktop configuration", error));
    }));
  }

  static async create(supervisor: AppServerSupervisor, legacy: LegacyConfigurationReader): Promise<AppServerConfigurationMainService> {
    let config = await supervisor.request(APP_SERVER_METHODS["config/read"], {});
    const legacySnapshot = legacy.read();
    const migration = legacyDesktopUpdate(config, legacySnapshot.document);
    if (migration) {
      await supervisor.request(APP_SERVER_METHODS["config/update"], {
        commandId: randomUUID(),
        expectedRevision: config.revision,
        products: { desktop: migration },
      });
      config = await supervisor.request(APP_SERVER_METHODS["config/read"], {});
    }
    await clearLegacyDesktopValues(legacy, legacySnapshot);
    return new AppServerConfigurationMainService(supervisor, config);
  }

  get onDidChange(): Event<IConfigurationSnapshot> {
    return this._onDidChange.event;
  }

  read(): IConfigurationSnapshot {
    return this.snapshot;
  }

  async update(request: IConfigurationUpdateRequest): Promise<IConfigurationSnapshot> {
    if (request.expectedRevision !== this.snapshot.revision) {
      throw new Error(`configuration revision conflict: expected ${request.expectedRevision}, actual ${this.snapshot.revision}`);
    }
    await this.supervisor.request(APP_SERVER_METHODS["config/update"], {
      commandId: randomUUID(),
      expectedRevision: request.expectedRevision,
      products: { desktop: desktopUpdate(validateConfigurationDocument(request.document)) },
    });
    return this.refresh();
  }

  private refresh(): Promise<IConfigurationSnapshot> {
    if (this.refreshPromise) return this.refreshPromise;
    this.refreshPromise = this.supervisor.request(APP_SERVER_METHODS["config/read"], {}).then(config => {
      const next = desktopSnapshot(config);
      const changed = next.revision !== this.snapshot.revision || JSON.stringify(next.document) !== JSON.stringify(this.snapshot.document);
      this.snapshot = next;
      if (changed) this._onDidChange.fire(next);
      return next;
    }).finally(() => {
      this.refreshPromise = undefined;
    });
    return this.refreshPromise;
  }
}

function desktopSnapshot(config: ConfigReadResult): IConfigurationSnapshot {
  const desktop = config.products.desktop;
  const values: Record<string, string | number | boolean> = {};
  assign(values, desktopKeys.colorTheme, desktop.colorTheme);
  assign(values, desktopKeys.accessibilitySupport, desktop.accessibilitySupport);
  assign(values, desktopKeys.reduceMotion, desktop.reduceMotion);
  assign(values, desktopKeys.reduceTransparency, desktop.reduceTransparency);
  assign(values, desktopKeys.underlineLinks, desktop.underlineLinks);
  assign(values, desktopKeys.hoverDelayMilliseconds, desktop.hoverDelayMilliseconds);
  assign(values, desktopKeys.reducedHoverDelayMilliseconds, desktop.reducedHoverDelayMilliseconds);
  assign(values, desktopKeys.sashSize, desktop.sashSize);
  assign(values, desktopKeys.sashHoverDelayMilliseconds, desktop.sashHoverDelayMilliseconds);
  return { revision: config.revision, document: { version: 1, values } };
}

function desktopUpdate(document: IConfigurationDocument): DesktopProductPreferencesUpdateDto {
  const supported = new Set(Object.values(desktopKeys));
  for (const key of Object.keys(document.values)) {
    if (!supported.has(key as (typeof desktopKeys)[keyof typeof desktopKeys])) throw new Error(`Desktop configuration key '${key}' is not supported by config.toml`);
  }
  return {
    colorTheme: stringOrNull(document, desktopKeys.colorTheme),
    accessibilitySupport: automaticOrNull(document, desktopKeys.accessibilitySupport),
    reduceMotion: automaticOrNull(document, desktopKeys.reduceMotion),
    reduceTransparency: automaticOrNull(document, desktopKeys.reduceTransparency),
    underlineLinks: booleanOrNull(document, desktopKeys.underlineLinks),
    hoverDelayMilliseconds: integerOrNull(document, desktopKeys.hoverDelayMilliseconds),
    reducedHoverDelayMilliseconds: integerOrNull(document, desktopKeys.reducedHoverDelayMilliseconds),
    sashSize: integerOrNull(document, desktopKeys.sashSize),
    sashHoverDelayMilliseconds: integerOrNull(document, desktopKeys.sashHoverDelayMilliseconds),
  };
}

function legacyDesktopUpdate(config: ConfigReadResult, legacy: IConfigurationDocument): DesktopProductPreferencesUpdateDto | undefined {
  const current = desktopSnapshot(config).document.values;
  const values = Object.fromEntries(Object.entries(legacy.values).filter(([key]) => current[key] === undefined));
  if (Object.keys(values).length === 0) return undefined;
  const supported = new Set(Object.values(desktopKeys));
  const filtered = Object.fromEntries(Object.entries(values).filter(([key]) => supported.has(key as (typeof desktopKeys)[keyof typeof desktopKeys])));
  return Object.keys(filtered).length === 0 ? undefined : desktopUpdate({ version: 1, values: filtered });
}

async function clearLegacyDesktopValues(legacy: LegacyConfigurationReader, snapshot: IConfigurationSnapshot): Promise<void> {
  const supported = new Set(Object.values(desktopKeys));
  const values = Object.fromEntries(Object.entries(snapshot.document.values).filter(([key]) => !supported.has(key as (typeof desktopKeys)[keyof typeof desktopKeys])));
  if (Object.keys(values).length === Object.keys(snapshot.document.values).length) return;
  await legacy.update({ expectedRevision: snapshot.revision, document: { version: 1, values } });
}

function assign(values: Record<string, string | number | boolean>, key: string, value: string | number | boolean | null | undefined): void {
  if (value !== undefined && value !== null) values[key] = value;
}

function stringOrNull(document: IConfigurationDocument, key: string): string | null {
  const value = document.values[key];
  if (value === undefined) return null;
  if (typeof value !== "string") throw new Error(`${key} must be a string`);
  return value;
}

function automaticOrNull(document: IConfigurationDocument, key: string): AutomaticPreferenceDto | null {
  const value = stringOrNull(document, key);
  if (value === null || value === "auto" || value === "off" || value === "on") return value;
  throw new Error(`${key} must be auto, off, or on`);
}

function booleanOrNull(document: IConfigurationDocument, key: string): boolean | null {
  const value = document.values[key];
  if (value === undefined) return null;
  if (typeof value !== "boolean") throw new Error(`${key} must be a boolean`);
  return value;
}

function integerOrNull(document: IConfigurationDocument, key: string): number | null {
  const value = document.values[key];
  if (value === undefined) return null;
  if (!Number.isSafeInteger(value)) throw new Error(`${key} must be an integer`);
  return value as number;
}

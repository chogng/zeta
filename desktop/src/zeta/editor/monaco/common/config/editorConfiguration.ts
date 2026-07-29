import {
  isMacintosh,
  isWindows,
} from "../../../../base/common/platform.js";
import type {
  IConfigurationChangeEvent,
  IConfigurationKey,
  IConfigurationService,
} from "../../../../platform/configuration/common/configuration.js";
import {
  ConfigurationsRegistry,
} from "../../../../platform/configuration/common/configurationRegistry.js";

export interface IMonacoEditorFontSettings {
  readonly fontFamily: string;
  readonly fontWeight: string;
  readonly fontSize: number;
  readonly fontLigatures: boolean | string;
  readonly fontVariations: boolean | string;
  readonly lineHeight: number;
  readonly letterSpacing: number;
}

export const MONACO_EDITOR_FONT_DEFAULTS =
  Object.freeze<IMonacoEditorFontSettings>({
    fontFamily: isMacintosh
      ? "Menlo, Monaco, 'Courier New', monospace"
      : isWindows
        ? "Consolas, 'Courier New', monospace"
        : "'Droid Sans Mono', monospace",
    fontWeight: "normal",
    fontSize: isMacintosh ? 12 : 14,
    fontLigatures: false,
    fontVariations: false,
    lineHeight: 0,
    letterSpacing: 0,
  });

export const MonacoEditorFontConfiguration = Object.freeze({
  fontFamily: ConfigurationsRegistry.registerConfiguration<string>({
    key: "editor.fontFamily",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.fontFamily,
    parse: parseFontFamily,
  }),
  fontWeight: ConfigurationsRegistry.registerConfiguration<string>({
    key: "editor.fontWeight",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.fontWeight,
    parse: parseFontWeight,
  }),
  fontSize: ConfigurationsRegistry.registerConfiguration<number>({
    key: "editor.fontSize",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.fontSize,
    parse: (value) => boundedNumber(value, "editor.fontSize", 6, 100),
  }),
  fontLigatures: ConfigurationsRegistry.registerConfiguration<
    boolean | string
  >({
    key: "editor.fontLigatures",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.fontLigatures,
    parse: (value) => booleanOrString(value, "editor.fontLigatures"),
  }),
  fontVariations: ConfigurationsRegistry.registerConfiguration<
    boolean | string
  >({
    key: "editor.fontVariations",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.fontVariations,
    parse: (value) => booleanOrString(value, "editor.fontVariations"),
  }),
  lineHeight: ConfigurationsRegistry.registerConfiguration<number>({
    key: "editor.lineHeight",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.lineHeight,
    parse: (value) => boundedNumber(value, "editor.lineHeight", 0, 150),
  }),
  letterSpacing: ConfigurationsRegistry.registerConfiguration<number>({
    key: "editor.letterSpacing",
    defaultValue: MONACO_EDITOR_FONT_DEFAULTS.letterSpacing,
    parse: (value) => boundedNumber(
      value,
      "editor.letterSpacing",
      -5,
      20,
    ),
  }),
});

const editorFontKeys: readonly IConfigurationKey<unknown>[] =
  Object.values(MonacoEditorFontConfiguration);

export function readMonacoEditorFontSettings(
  service?: IConfigurationService,
): IMonacoEditorFontSettings {
  return {
    fontFamily: value(service, MonacoEditorFontConfiguration.fontFamily),
    fontWeight: value(service, MonacoEditorFontConfiguration.fontWeight),
    fontSize: value(service, MonacoEditorFontConfiguration.fontSize),
    fontLigatures: value(
      service,
      MonacoEditorFontConfiguration.fontLigatures,
    ),
    fontVariations: value(
      service,
      MonacoEditorFontConfiguration.fontVariations,
    ),
    lineHeight: value(service, MonacoEditorFontConfiguration.lineHeight),
    letterSpacing: value(
      service,
      MonacoEditorFontConfiguration.letterSpacing,
    ),
  };
}

export function affectsMonacoEditorFontConfiguration(
  event: IConfigurationChangeEvent,
): boolean {
  return editorFontKeys.some((key) => event.affectsConfiguration(key));
}

function value<T>(
  service: IConfigurationService | undefined,
  key: IConfigurationKey<T>,
): T {
  return service?.getValue(key) ?? key.defaultValue;
}

function parseFontFamily(value: unknown): string {
  if (typeof value !== "string") {
    throw new TypeError("editor.fontFamily must be a string");
  }
  return value.trim() || MONACO_EDITOR_FONT_DEFAULTS.fontFamily;
}

function parseFontWeight(value: unknown): string {
  if (typeof value !== "string") {
    throw new TypeError("editor.fontWeight must be a string");
  }
  if (value === "normal" || value === "bold") return value;
  if (/^\d{1,4}$/.test(value)) {
    return String(Math.min(1000, Math.max(1, Number(value))));
  }
  throw new RangeError(
    "editor.fontWeight must be normal, bold, or a number from 1 to 1000",
  );
}

function boundedNumber(
  value: unknown,
  key: string,
  minimum: number,
  maximum: number,
): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`${key} must be a finite number`);
  }
  return Math.min(maximum, Math.max(minimum, value));
}

function booleanOrString(
  value: unknown,
  key: string,
): boolean | string {
  if (typeof value === "boolean" || typeof value === "string") {
    return value;
  }
  throw new TypeError(`${key} must be a boolean or string`);
}

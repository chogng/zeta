import { ConfigurationsRegistry } from "../../../../platform/configuration/common/configurationRegistry.js";

export const MinimumSashSize = 1;
export const MaximumSashSize = 20;
export const MinimumSashHoverDelay = 0;
export const MaximumSashHoverDelay = 2_000;

/** Typed configuration keys owned by the Workbench Sash contribution. */
export const SashConfiguration = Object.freeze({
  size: ConfigurationsRegistry.registerConfiguration<number>({
    key: "workbench.sash.size",
    defaultValue: 4,
    parse: (value) => parseNumberInRange(
      value,
      "workbench.sash.size",
      MinimumSashSize,
      MaximumSashSize,
    ),
  }),
  hoverDelay: ConfigurationsRegistry.registerConfiguration<number>({
    key: "workbench.sash.hoverDelay",
    defaultValue: 300,
    parse: (value) => parseNumberInRange(
      value,
      "workbench.sash.hoverDelay",
      MinimumSashHoverDelay,
      MaximumSashHoverDelay,
    ),
  }),
});

function parseNumberInRange(
  value: unknown,
  key: string,
  minimum: number,
  maximum: number,
): number {
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < minimum ||
    value > maximum
  ) {
    throw new RangeError(
      `${key} must be a finite number between ${minimum} and ${maximum}`,
    );
  }
  return value;
}

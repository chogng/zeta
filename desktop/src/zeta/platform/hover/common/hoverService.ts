import type { HoverDelegateSetupOptions, IHoverDelegate, IManagedHover as IBaseManagedHover } from "../../../base/browser/ui/hover/hoverDelegate.js";
import { ConfigurationsRegistry } from "../../configuration/common/configurationRegistry.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export const MinimumHoverDelay = 0;
export const MaximumHoverDelay = 2_000;

/** Typed configuration keys owned by the Workbench Hover service. */
export const HoverConfiguration = Object.freeze({
  delay: ConfigurationsRegistry.registerConfiguration<number>({
    key: "workbench.hover.delay",
    defaultValue: 300,
    parse: (value) => parseHoverDelay(value, "workbench.hover.delay"),
  }),
  reducedDelay: ConfigurationsRegistry.registerConfiguration<number>({
    key: "workbench.hover.reducedDelay",
    defaultValue: 30,
    parse: (value) => parseHoverDelay(
      value,
      "workbench.hover.reducedDelay",
    ),
  }),
});

/** Selects the Workbench policy used before an automatic Hover is shown. */
export type HoverDelayMode = "standard" | "reduced" | "instant";

/** Caller-owned description of one target and its managed Hover content. */
export interface HoverSetupOptions extends HoverDelegateSetupOptions {
  readonly delay?: HoverDelayMode;
}

/** Handle returned to callers for updating or explicitly controlling a Hover. */
export type IManagedHover = IBaseManagedHover;

/**
 * Coordinates managed Hovers inside one Workbench window.
 *
 * Implementations own product delay policy and global overlay coordination;
 * callers retain ownership of the returned managed Hover handles.
 */
export interface IHoverService extends IHoverDelegate {
  setupHover(options: HoverSetupOptions): IManagedHover;
  showHover(options: HoverSetupOptions): IManagedHover;
  hideHover(): void;
}

export const IHoverService = createServiceIdentifier<IHoverService>("hoverService");

function parseHoverDelay(value: unknown, key: string): number {
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < MinimumHoverDelay ||
    value > MaximumHoverDelay
  ) {
    throw new RangeError(
      `${key} must be a finite number between ${MinimumHoverDelay} and ${MaximumHoverDelay}`,
    );
  }
  return value;
}

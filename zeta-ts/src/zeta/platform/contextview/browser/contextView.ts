import type {
  IContextViewProvider,
} from "../../../base/browser/ui/contextview/contextview.js";
import {
  createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

/**
 * Provides the shared transient-view host for one Workbench container.
 *
 * Consumers use the provider contract while the Workbench owns where browser
 * overlays are mounted and which platform, theme, and typography they inherit.
 */
export interface IContextViewService extends IContextViewProvider {
  readonly container: HTMLElement;
}

export const IContextViewService =
  createServiceIdentifier<IContextViewService>("contextViewService");

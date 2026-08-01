import type { IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import type {
  INativeHostApi,
} from "../../platform/native/common/nativeHost.js";
import {
  createServiceIdentifier,
} from "../../platform/instantiation/common/instantiation.js";

/** Renderer API available to commands executing in a workbench window. */
export const IRendererApiService =
  createServiceIdentifier<IRendererHost>("rendererApiService");

/** Native window capabilities available only in Electron workbenches. */
export const INativeHostService =
  createServiceIdentifier<INativeHostApi>("nativeHostService");

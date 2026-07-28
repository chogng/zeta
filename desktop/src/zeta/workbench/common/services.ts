import type {
  ZetaRendererApi,
} from "../../platform/app-server/common/renderer-api.js";
import type {
  INativeHostApi,
} from "../../platform/native/common/nativeHost.js";
import {
  createServiceIdentifier,
} from "../../platform/instantiation/common/instantiation.js";

/** Renderer API available to commands executing in a workbench window. */
export const IRendererApiService =
  createServiceIdentifier<ZetaRendererApi>("rendererApiService");

/** Native window capabilities available only in Electron workbenches. */
export const INativeHostService =
  createServiceIdentifier<INativeHostApi>("nativeHostService");

import type {
  ZetaRendererApi,
} from "../../platform/app-server/common/renderer-api.js";
import {
  createServiceIdentifier,
} from "../../platform/instantiation/common/instantiation.js";

/** Renderer API available to commands executing in a workbench window. */
export const IRendererApiService =
  createServiceIdentifier<ZetaRendererApi>("rendererApiService");

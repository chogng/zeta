import { createServiceIdentifier, } from "../../platform/instantiation/common/instantiation.js";
/** Renderer API available to commands executing in a workbench window. */
export const IRendererApiService = createServiceIdentifier("rendererApiService");
/** Native window capabilities available only in Electron workbenches. */
export const INativeHostService = createServiceIdentifier("nativeHostService");

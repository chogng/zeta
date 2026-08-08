export const productIds = [
  "code",
  "academic",
  "complete",
] as const;

export type ProductId = typeof productIds[number];

export const rendererEntryNames = [
  "workbench-code",
  "workbench-academic",
  "workbench-complete",
] as const;

export type RendererEntryName = typeof rendererEntryNames[number];

/** Build-time identity and presentation owned by one Zeta product edition. */
export interface ProductConfiguration {
  readonly id: ProductId;
  readonly name: string;
  /** Stable installer/application identity; never derive this from `name`. */
  readonly applicationId: string;
  /** Stable directory name below the platform application-data root. */
  readonly userDataFolderName: string;
  /** Stable renderer storage namespace for this product edition. */
  readonly storageNamespace: string;
  readonly rendererEntry: RendererEntryName;
}

export const ZetaDesktopProduct: ProductConfiguration = {
  id: "code",
  name: "Zeta",
  applicationId: "com.zeta.desktop.code",
  userDataFolderName: "Zeta",
  storageNamespace: "code",
  rendererEntry: "workbench-code",
};

export const AcademicProduct: ProductConfiguration = {
  id: "academic",
  name: "Zeta Academic",
  applicationId: "com.zeta.desktop.academic",
  userDataFolderName: "Zeta Academic",
  storageNamespace: "academic",
  rendererEntry: "workbench-academic",
};

export const CompleteProduct: ProductConfiguration = {
  id: "complete",
  name: "Zeta Complete",
  applicationId: "com.zeta.desktop.complete",
  userDataFolderName: "Zeta Complete",
  storageNamespace: "complete",
  rendererEntry: "workbench-complete",
};

const products: Readonly<Record<ProductId, ProductConfiguration>> = {
  code: ZetaDesktopProduct,
  academic: AcademicProduct,
  complete: CompleteProduct,
};

/** Resolves the selected Electron Desktop build, defaulting local and legacy builds to Zeta. */
export function resolveProductId(value: string | undefined): ProductId {
  if (value === undefined || value.length === 0) return "code";
  if (isProductId(value)) return value;
  throw new TypeError(
    `Unknown Zeta product '${value}'. Expected ${productIds.join(", ")}`,
  );
}

export function getProductConfiguration(
  id: ProductId,
): ProductConfiguration {
  return products[id];
}

function isProductId(value: string): value is ProductId {
  return (productIds as readonly string[]).includes(value);
}

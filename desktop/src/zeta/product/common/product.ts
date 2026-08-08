export const productIds = [
  "code",
  "academic",
] as const;

export type ProductId = typeof productIds[number];

export const rendererEntryNames = [
  "workbench-code",
  "workbench-academic",
] as const;

export type RendererEntryName = typeof rendererEntryNames[number];

/** Product-owned renderer entry for an optional dedicated Sessions window. */
export interface DedicatedSessionsConfiguration {
  readonly rendererEntry: string;
}

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
  /** Omitted when the product keeps all of its work inside the regular Workbench. */
  readonly dedicatedSessions?: DedicatedSessionsConfiguration;
}

export const ZetaDesktopProduct: ProductConfiguration = {
  id: "code",
  name: "Zeta",
  applicationId: "com.zeta.desktop.code",
  userDataFolderName: "Zeta",
  storageNamespace: "code",
  rendererEntry: "workbench-code",
  dedicatedSessions: {
    rendererEntry: "sessions-code",
  },
};

export const AcademicProduct: ProductConfiguration = {
  id: "academic",
  name: "Zeta Academic",
  applicationId: "com.zeta.desktop.academic",
  userDataFolderName: "Zeta Academic",
  storageNamespace: "academic",
  rendererEntry: "workbench-academic",
};

const products: Readonly<Record<ProductId, ProductConfiguration>> = {
  code: ZetaDesktopProduct,
  academic: AcademicProduct,
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

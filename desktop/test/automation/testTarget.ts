export type AppServerTestMode = "disabled" | "required";
export type DesktopProduct = "academic" | "code";

export type PlaywrightTarget =
  | {
      readonly kind: "browser";
      readonly appServerMode: AppServerTestMode;
      readonly baseURL: string;
      readonly product: DesktopProduct;
    }
  | {
      readonly kind: "electron";
      readonly appServerMode: AppServerTestMode;
      readonly product: DesktopProduct;
    };

export function playwrightTargetForProject(projectName: string, baseURL: string | undefined): PlaywrightTarget {
  switch (projectName) {
    case "browser-ui":
      return { kind: "browser", appServerMode: "disabled", baseURL: requiredBaseURL(baseURL, projectName), product: testProduct() };
    case "browser-app-server":
      return { kind: "browser", appServerMode: "required", baseURL: requiredBaseURL(baseURL, projectName), product: testProduct() };
    case "electron-ui":
      return { kind: "electron", appServerMode: "disabled", product: testProduct() };
    case "electron-academic-ui":
      return { kind: "electron", appServerMode: "disabled", product: "academic" };
    case "electron-app-server":
    case "electron-editor-app-server":
    case "electron-pdf-corpus-app-server":
      return { kind: "electron", appServerMode: "required", product: testProduct() };
    default:
      throw new Error(`Unsupported Playwright project: ${projectName}`);
  }
}

function testProduct(): DesktopProduct {
  return process.env.ZETA_PRODUCT === "academic" ? "academic" : "code";
}

function requiredBaseURL(baseURL: string | undefined, projectName: string): string {
  if (baseURL === undefined) {
    throw new Error(`Playwright project '${projectName}' requires a baseURL`);
  }
  return baseURL;
}

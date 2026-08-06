export type AppServerTestMode = "disabled" | "required";
export type DesktopProduct = "academic" | "code" | "complete";

export type PlaywrightTarget =
  | {
      readonly kind: "browser";
      readonly appServerMode: AppServerTestMode;
      readonly baseURL: string;
    }
  | {
      readonly kind: "electron";
      readonly appServerMode: AppServerTestMode;
      readonly product: DesktopProduct;
    };

export function playwrightTargetForProject(projectName: string, baseURL: string | undefined): PlaywrightTarget {
  switch (projectName) {
    case "browser-ui":
      return { kind: "browser", appServerMode: "disabled", baseURL: requiredBaseURL(baseURL, projectName) };
    case "browser-app-server":
      return { kind: "browser", appServerMode: "required", baseURL: requiredBaseURL(baseURL, projectName) };
    case "electron-ui":
      return { kind: "electron", appServerMode: "disabled", product: "code" };
    case "electron-app-server":
    case "electron-editor-code-app-server":
      return { kind: "electron", appServerMode: "required", product: "code" };
    case "electron-editor-academic-app-server":
      return { kind: "electron", appServerMode: "required", product: "academic" };
    case "electron-editor-complete-app-server":
      return { kind: "electron", appServerMode: "required", product: "complete" };
    default:
      throw new Error(`Unsupported Playwright project: ${projectName}`);
  }
}

function requiredBaseURL(baseURL: string | undefined, projectName: string): string {
  if (baseURL === undefined) {
    throw new Error(`Playwright project '${projectName}' requires a baseURL`);
  }
  return baseURL;
}

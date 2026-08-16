import { AppServerRemoteError } from "../../../../platform/app-server/common/appServerError.js";

export function gitErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  const errorName = error instanceof AppServerRemoteError ? error.errorName : message;
  if (/GitNotRepository/.test(errorName)) return "The open folder is not a Git repository.";
  if (/GitUnavailable/.test(errorName)) {
    return "Git is unavailable for this folder. Trust the folder to enable Source Control, or continue in Restricted Mode.";
  }
  return error instanceof Error ? error.message : "Git operation failed.";
}

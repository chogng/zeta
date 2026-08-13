import { type IDisposable } from "../../base/common/lifecycle.js";
import { type IRendererHost } from "../../platform/renderer/common/rendererHost.js";
import { type IFileService } from "../../platform/files/common/files.js";
import { type IWorkspaceContextService } from "../../platform/workspace/common/workspace.js";
import { type ServiceCollection } from "../../platform/instantiation/common/instantiation.js";
import { type ITerminalService } from "../services/terminal/common/terminal.js";

export interface WorkbenchServiceContributionContext {
  readonly services: ServiceCollection;
  readonly rendererHost: IRendererHost;
  readonly fileService: IFileService;
  readonly workspaceContext: IWorkspaceContextService;
  readonly terminalService: ITerminalService;
  readonly own: <T extends IDisposable>(value: T) => T;
}

export type WorkbenchServiceContribution = (context: WorkbenchServiceContributionContext) => void;

const contributions: WorkbenchServiceContribution[] = [];

/** Registers services owned by a statically selected product contribution bundle. */
export function registerWorkbenchServiceContribution(contribution: WorkbenchServiceContribution): void {
  if (typeof contribution !== "function") throw new TypeError("Workbench service contribution must be a function");
  contributions.push(contribution);
}

export function installWorkbenchServiceContributions(context: WorkbenchServiceContributionContext): void {
  for (const contribution of contributions) contribution(context);
}

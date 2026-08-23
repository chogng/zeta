import { IDebugAdapterProcessService } from "../../../../platform/debug/common/debugAdapterProcessService.js";
import { IFileService } from "../../../../platform/files/common/files.js";
import { ILogService } from "../../../../platform/log/common/logService.js";
import { IStorageService } from "../../../../platform/storage/common/storage.js";
import { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { ITaskService } from "../../tasks/common/taskService.js";
import { ITerminalService } from "../../terminal/common/terminal.js";
import { IDebugConsoleService } from "../common/debugConsoleService.js";
import { DebugAdapterFactoriesRegistry } from "../common/debugAdapterFactory.js";
import { IDebugService } from "../common/debugService.js";
import { DebugConsoleService } from "./debugConsoleService.js";
import { DebugService } from "./debugService.js";

registerWorkbenchServiceContribution({
	service: IDebugService,
	dependencies: [IFileService, IWorkspaceContextService, ITerminalService, IStorageService, ITaskService, ILogService],
	install: context => {
		const service = context.own(new DebugService(context.services.get(IFileService), context.services.get(IWorkspaceContextService), context.services.getOptional(IDebugAdapterProcessService), context.services.get(ITerminalService), context.services.get(IStorageService), context.services.get(ITaskService), DebugAdapterFactoriesRegistry, context.services.get(ILogService)));
		context.services.set(IDebugConsoleService, context.own(new DebugConsoleService(service)));
		return service;
	},
});

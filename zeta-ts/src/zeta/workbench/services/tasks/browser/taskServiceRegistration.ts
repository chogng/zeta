import { IFileService } from "../../../../platform/files/common/files.js";
import { ILogService } from "../../../../platform/log/common/logService.js";
import { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { IOutputService } from "../../output/common/outputService.js";
import { ITerminalService } from "../../terminal/common/terminal.js";
import { ITaskService } from "../common/taskService.js";
import { TaskService } from "./taskService.js";

registerWorkbenchServiceContribution({
	service: ITaskService,
	dependencies: [IFileService, IWorkspaceContextService, ITerminalService, IOutputService, ILogService],
	install: context => context.own(new TaskService(context.services.get(IFileService), context.services.get(IWorkspaceContextService), context.services.get(ITerminalService), context.services.get(IOutputService), context.services.get(ILogService))),
});

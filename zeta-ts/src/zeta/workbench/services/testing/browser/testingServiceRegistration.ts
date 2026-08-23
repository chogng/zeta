import { ILogService } from "../../../../platform/log/common/logService.js";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { ITaskService } from "../../tasks/common/taskService.js";
import { ITestingService } from "../common/testingService.js";
import { TestingService } from "./testingService.js";

registerWorkbenchServiceContribution({
	service: ITestingService,
	dependencies: [ITaskService, ILogService],
	install: context => context.own(new TestingService(context.services.get(ITaskService), context.services.get(ILogService))),
});

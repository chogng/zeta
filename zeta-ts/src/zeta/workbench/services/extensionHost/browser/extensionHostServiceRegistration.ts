import { IExtensionHostApi } from "../../../../platform/extensionHost/common/extensionHostApi.js";
import { ILogService } from "../../../../platform/log/common/logService.js";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { ILanguageFeaturesService } from "../../language/common/languageFeaturesService.js";
import { IOutputService } from "../../output/common/outputService.js";
import { ITaskService } from "../../tasks/common/taskService.js";
import { ITestingService } from "../../testing/common/testingService.js";
import { IExtensionHostService } from "../common/extensionHostService.js";
import { AppServerExtensionHostService } from "./appServerExtensionHostService.js";

registerWorkbenchServiceContribution({
	service: IExtensionHostService,
	dependencies: [IExtensionHostApi, ILogService, ILanguageFeaturesService, ITaskService, ITestingService, IOutputService],
	install: context => {
		const service = context.own(new AppServerExtensionHostService({
			api: context.services.get(IExtensionHostApi),
			languageFeatures: context.services.get(ILanguageFeaturesService),
			tasks: context.services.get(ITaskService),
			testing: context.services.get(ITestingService),
			output: context.services.get(IOutputService),
		}));
		const ready = service.start();
		context.blockRestorationUntil(ready);
		void ready.catch(error => context.services.get(ILogService).error("extensionHost", "Executable Extension Host activation failed", error));
		return service;
	},
});

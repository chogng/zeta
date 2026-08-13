import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { AppServerExtensionHostService } from "../../../services/extensionHost/browser/appServerExtensionHostService.js";
import { IExtensionHostService } from "../../../services/extensionHost/common/extensionHostService.js";
import { ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import { ITaskService } from "../../../services/tasks/common/taskService.js";
import { ITestingService } from "../../../services/testing/common/testingService.js";

registerWorkbenchServiceContribution(context => {
  const service = context.own(new AppServerExtensionHostService({
    api: context.rendererHost.extensionHost,
    languageFeatures: context.services.get(ILanguageFeaturesService),
    tasks: context.services.get(ITaskService),
    testing: context.services.get(ITestingService),
  }));
  context.services.set(IExtensionHostService, service);
  const ready = service.start();
  context.blockRestorationUntil(ready);
  void ready.catch(error => console.error("Executable Extension Host activation failed", error));
});

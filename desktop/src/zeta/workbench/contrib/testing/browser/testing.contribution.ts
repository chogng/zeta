import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { ITestingService } from "../../../services/testing/common/testingService.js";
import { ITerminalService } from "../../../services/terminal/common/terminal.js";
import { IViewsService } from "../../../services/views/browser/viewsService.js";
import { TESTING_VIEW_ID } from "../common/testing.js";
import { TestingViewPane } from "./testingViewPane.js";
import "./testingActions.js";
import "./media/testing.css";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { ITaskService } from "../../../services/tasks/common/taskService.js";
import { TestingService } from "../../../services/testing/browser/testingService.js";

export function registerTestingView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.Testing, title: "Testing", location: ViewContainerLocation.Sidebar, icon: lxiconsLibrary.check, order: 4 });
  registry.registerStaticViews(WorkbenchViewContainerId.Testing, [{
    id: TESTING_VIEW_ID,
    title: "Testing",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(TestingViewPane, { serviceDependencies: [ITestingService, ITerminalService, IViewsService] }),
  }]);
}

registerTestingView();
registerWorkbenchServiceContribution(context => context.services.set(ITestingService, context.own(new TestingService(context.services.get(ITaskService)))));

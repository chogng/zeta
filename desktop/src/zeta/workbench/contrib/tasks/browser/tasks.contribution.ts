import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { ITaskService } from "../../../services/tasks/common/taskService.js";
import { IOutputService } from "../../../services/output/common/outputService.js";
import { ITerminalService } from "../../../services/terminal/common/terminal.js";
import { IViewsService } from "../../../services/views/browser/viewsService.js";
import { TASKS_VIEW_ID } from "../common/tasks.js";
import { TasksViewPane } from "./tasksViewPane.js";
import "./taskActions.js";
import "./media/tasks.css";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { TaskService } from "../../../services/tasks/browser/taskService.js";

/** Contributes the Code task catalog as its own Panel destination. */
export function registerTasksView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Tasks,
    title: "Tasks",
    location: ViewContainerLocation.Panel,
    order: 2.5,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Tasks, [{
    id: TASKS_VIEW_ID,
    title: "Tasks",
    order: 2,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(TasksViewPane, { serviceDependencies: [ITaskService, IViewsService, ITerminalService] }),
  }]);
}

registerTasksView();
registerWorkbenchServiceContribution(context => context.services.set(ITaskService, context.own(new TaskService(context.fileService, context.workspaceContext, context.terminalService, context.services.get(IOutputService)))));

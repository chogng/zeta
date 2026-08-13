import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { ILanguageDiagnosticsService } from "../../../services/language/common/languageDiagnosticsService.js";
import { ProblemsViewPane } from "./problemsViewPane.js";
import "./media/problems.css";

export const PROBLEMS_VIEW_ID = "zeta.problems";

/** Registers the Workbench-owned Problems panel. */
export function registerProblemsView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({
    id: WorkbenchViewContainerId.Problems,
    title: "Problems",
    location: ViewContainerLocation.Panel,
    order: 1,
  });
  registry.registerStaticViews(WorkbenchViewContainerId.Problems, [{
    id: PROBLEMS_VIEW_ID,
    title: "Problems",
    order: 1,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(ProblemsViewPane, {
      serviceDependencies: [ILanguageDiagnosticsService, IEditorPart],
    }),
  }]);
}

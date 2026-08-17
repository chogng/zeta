import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { isCancellationError } from "../../../../base/common/cancellation.js";
import { type LanguageWorkspaceEdit } from "../../../../editor/common/languages/languageWorkspaceEdit.js";
import { ITextModelService } from "../../../../editor/common/services/textModelService.js";
import { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { IFileService } from "../../../../platform/files/common/files.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../common/contributions.js";
import { ViewContainerLocation, type WorkbenchViewRegistry, WorkbenchViewContainerId, ViewsRegistry } from "../../../common/views.js";
import { IViewsService } from "../../../services/views/browser/viewsService.js";
import { IWorkingCopyService } from "../../../services/workingCopy/common/workingCopyService.js";
import { IBulkEditService } from "../common/bulkEdit.js";
import { createBulkEditPreview } from "./preview/bulkEditPreview.js";
import { BulkEditPreviewPane, BULK_EDIT_VIEW_ID } from "./preview/bulkEditPreviewPane.js";
import "./media/bulkEdit.css";

/** Registers the Workbench panel that hosts the transient bulk-edit preview. */
export function registerBulkEditView(registry: WorkbenchViewRegistry = ViewsRegistry): void {
  registry.registerStaticViewContainer({ id: WorkbenchViewContainerId.BulkEdit, title: "Refactor Preview", location: ViewContainerLocation.Panel, order: 2.75 });
  registry.registerStaticViews(WorkbenchViewContainerId.BulkEdit, [{
    id: BULK_EDIT_VIEW_ID,
    title: "Refactor Preview",
    order: 1,
    hideByDefault: true,
    canToggleVisibility: false,
    ctorDescriptor: new SyncDescriptor(BulkEditPreviewPane),
  }]);
}

/** Connects the bulk-edit service to the transient preview pane. */
export class BulkEditPreviewContribution extends DisposableOwner {
  private activeSession: PreviewSession | undefined;

  constructor(private readonly bulkEdits: IBulkEditService, private readonly views: IViewsService, private readonly files: IFileService, private readonly models: ITextModelService, private readonly workingCopies: IWorkingCopyService, private readonly dialogs: IDialogService) {
    super();
    this.own(bulkEdits.setPreviewHandler((edit, signal) => this.preview(edit, signal)));
    this.defer(() => this.activeSession?.controller.abort());
  }

  private async preview(edit: LanguageWorkspaceEdit, signal: AbortSignal): Promise<LanguageWorkspaceEdit | undefined> {
    const view = this.views.openView(BULK_EDIT_VIEW_ID);
    if (!(view instanceof BulkEditPreviewPane)) throw new Error("Bulk edit preview view is not available");
    if (this.activeSession) {
      const confirmed = await this.dialogs.confirm({
        title: "Refactor Preview",
        message: "Another refactoring is being previewed.",
        detail: "Continue to discard the previous refactoring and preview this one?",
        primaryButton: "Continue",
        cancelButton: "Cancel",
      });
      if (!confirmed) return undefined;
      this.activeSession.controller.abort();
      view.cancelInput();
    }
    const controller = new AbortController();
    const abort = (): void => controller.abort();
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) controller.abort();
    const session = { controller };
    this.activeSession = session;
    try {
      const model = await createBulkEditPreview(edit, { files: this.files, models: this.models, workingCopies: this.workingCopies }, controller.signal);
      return await view.setInput(model, controller.signal);
    } catch (error) {
      if (isCancellationError(error) || controller.signal.aborted) return undefined;
      throw error;
    } finally {
      signal.removeEventListener("abort", abort);
      if (this.activeSession === session) this.activeSession = undefined;
    }
  }
}

registerBulkEditView();
interface PreviewSession {
  readonly controller: AbortController;
}

registerWorkbenchContribution("workbench.contrib.bulkEditPreview", WorkbenchPhase.BlockRestore, accessor => new BulkEditPreviewContribution(accessor.get(IBulkEditService), accessor.get(IViewsService), accessor.get(IFileService), accessor.get(ITextModelService), accessor.get(IWorkingCopyService), accessor.get(IDialogService)));

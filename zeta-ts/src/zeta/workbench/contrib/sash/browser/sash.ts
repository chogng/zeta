import { SashSettingsBinding } from "../../../../base/browser/ui/sash/sash.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { SashConfiguration } from "../common/sash.js";

const MinimumDragAreaSize = 4;
const MaximumHoverFeedbackSize = 8;

/** Projects persisted Workbench Sash preferences into one window. */
export class SashSettingsController extends DisposableOwner {
  private readonly configurationService: IConfigurationService;
  private readonly binding: SashSettingsBinding;

  constructor(
    configurationService: IConfigurationService,
    root: HTMLElement,
  ) {
    super();
    this.configurationService = configurationService;
    this.binding = this.own(new SashSettingsBinding(root));
    this.own(configurationService.onDidChangeConfiguration((event) => {
      if (
        event.affectsConfiguration(SashConfiguration.size) ||
        event.affectsConfiguration(SashConfiguration.hoverDelay)
      ) {
        this.apply();
      }
    }));
    this.apply();
  }

  private apply(): void {
    const configuredSize = this.configurationService.getValue(
      SashConfiguration.size,
    );
    this.binding.update({
      dragAreaSize: Math.max(configuredSize, MinimumDragAreaSize),
      hoverFeedbackSize: Math.min(
        configuredSize,
        MaximumHoverFeedbackSize,
      ),
      hoverDelay: this.configurationService.getValue(
        SashConfiguration.hoverDelay,
      ),
    });
  }
}

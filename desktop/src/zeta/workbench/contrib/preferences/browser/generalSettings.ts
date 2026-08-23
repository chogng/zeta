import "./media/generalSettings.css";
import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import type { IContextViewProvider } from "../../../../base/browser/ui/contextview/contextview.js";
import { SelectBox } from "../../../../base/browser/ui/selectbox/selectbox.js";
import { Checkbox, Toggle } from "../../../../base/browser/ui/toggle/toggle.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { AccessibilityConfiguration, type AccessibilityReductionConfiguration, type AccessibilitySupportConfiguration } from "../../../../platform/accessibility/common/accessibility.js";
import type { IConfigurationKey, IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { HoverConfiguration, MaximumHoverDelay, MinimumHoverDelay } from "../../../../platform/hover/common/hoverService.js";
import { MaximumSashHoverDelay, MaximumSashSize, MinimumSashHoverDelay, MinimumSashSize, SashConfiguration } from "../../sash/common/sash.js";
import { SettingsTree } from "./settingsTree.js";
import { SettingsTreeModel, type SettingsTreeNode } from "./settingsTreeModels.js";
import type { WorkbenchModeId } from "../../../../product/common/workbenchMode.js";
import type { IWorkbenchModeService } from "../../../services/workbenchMode/common/workbenchModeService.js";
import { WorkbenchConfiguration } from "../../../common/configuration.js";

type GeneralControl = HTMLInputElement | SelectBox | Toggle;

/** Core application preferences that are independent of one editor or feature domain. */
export class GeneralSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly controls = new Map<string, GeneralControl>();
  private readonly status: HTMLParagraphElement;

  constructor(container: HTMLElement, private readonly configurationService: IConfigurationService, private readonly contextViewProvider: IContextViewProvider, private readonly workbenchModeService: IWorkbenchModeService) {
    super();
    const ownerDocument = container.ownerDocument;
    this.element = h(ownerDocument, "div");
    this.element.className = "zeta-general-settings";
    container.append(this.element);
    const model = this.own(new SettingsTreeModel<HTMLElement>());
    model.setChildren([
      this.createGroup("mode", "Workbench Mode", "Choose the capabilities assembled for this window.", [
        this.createWorkbenchModeSetting(),
      ]),
      this.createGroup("accessibility", "Accessibility", "Adapt interaction and presentation to accessibility needs.", [
        this.createSelectSetting({
          key: AccessibilityConfiguration.editorAccessibilitySupport,
          label: "Screen reader optimization",
          description: "Let the operating system decide, or explicitly enable or disable optimized editor accessibility behavior.",
          options: triStateOptions<AccessibilitySupportConfiguration>(),
        }),
        this.createSelectSetting({
          key: AccessibilityConfiguration.reduceMotion,
          label: "Reduce motion",
          description: "Limit non-essential animation throughout the Workbench.",
          options: triStateOptions<AccessibilityReductionConfiguration>(),
        }),
        this.createSelectSetting({
          key: AccessibilityConfiguration.reduceTransparency,
          label: "Reduce transparency",
          description: "Prefer opaque surfaces where the active theme supports them.",
          options: triStateOptions<AccessibilityReductionConfiguration>(),
        }),
        this.createToggleSetting(AccessibilityConfiguration.underlineLinks, "Always underline links", "Keep link affordances visible without requiring hover or focus."),
      ]),
      this.createGroup("interaction", "Interaction", "Tune common pointer feedback and resize affordances.", [
        this.createNumberSetting(HoverConfiguration.delay, "Hover delay", "Milliseconds before standard managed hovers appear.", MinimumHoverDelay, MaximumHoverDelay),
        this.createNumberSetting(HoverConfiguration.reducedDelay, "Fast hover delay", "Milliseconds used for controls that request reduced-delay hover feedback.", MinimumHoverDelay, MaximumHoverDelay),
        this.createNumberSetting(SashConfiguration.size, "Resize handle size", "Width in pixels of Workbench resize handles.", MinimumSashSize, MaximumSashSize),
        this.createNumberSetting(SashConfiguration.hoverDelay, "Resize handle hover delay", "Milliseconds before resize handles show hover feedback.", MinimumSashHoverDelay, MaximumSashHoverDelay),
      ]),
    ]);
    const tree = this.own(new SettingsTree(this.element, {
      model,
      rootClassName: "zeta-general-settings-tree",
      groupClassName: "zeta-general-settings-group",
      groupDescriptionClassName: "zeta-general-settings-group-description",
      itemsClassName: "zeta-general-settings-list",
      renderItem: (item) => item.value,
    }));
    this.status = h(ownerDocument, "p");
    this.status.className = "zeta-general-settings-status";
    this.status.setAttribute("role", "status");
    this.status.setAttribute("aria-live", "polite");
    this.element.append(this.status);
    this.syncControls();
    this.own(configurationService.onDidChangeConfiguration(() => this.syncControls()));
  }

  private createGroup(id: string, title: string, description: string, settings: readonly HTMLElement[]): SettingsTreeNode<HTMLElement> {
    const groupId = `general.group.${id}`;
    return {
      element: { kind: "group", id: groupId, title, description },
      children: settings.map((setting, index) => this.createTreeItem(groupId, setting, index)),
    };
  }

  private createTreeItem(groupId: string, element: HTMLElement, index: number): SettingsTreeNode<HTMLElement> {
    const configurationKey = element.querySelector<HTMLElement>("[data-configuration-key]")?.dataset.configurationKey;
    const title = element.querySelector(".zeta-general-setting-title")?.textContent?.trim();
    const description = element.querySelector(".zeta-general-setting-description")?.textContent?.trim() ?? "";
    if (!title) throw new TypeError(`General setting '${configurationKey ?? index}' must have a title`);
    return {
      element: {
        kind: "item",
        id: `${groupId}.item.${configurationKey ?? index}`,
        title,
        description,
        keywords: configurationKey ? [configurationKey] : undefined,
        value: element,
      },
    };
  }

  private createToggleSetting(key: IConfigurationKey<boolean>, label: string, description: string): HTMLElement {
    const copy = this.createSettingCopy(label, description);
    const host = h(this.element.ownerDocument, "span");
    const checkbox = this.own(new Checkbox(host, {
      ariaLabel: label,
      content: copy,
      contentPlacement: "before-control",
    }));
    checkbox.element.classList.add("zeta-general-setting", "zeta-general-toggle-setting");
    checkbox.input.dataset.configurationKey = key.key;
    this.controls.set(key.key, checkbox);
    this.own(checkbox.onDidChange(checked => void this.updateConfiguration(key, checked)));
    return checkbox.element;
  }

  private createWorkbenchModeSetting(): HTMLElement {
    const setting = h(this.element.ownerDocument, "div");
    setting.className = "zeta-general-setting";
    const select = this.own(new SelectBox(setting, {
      options: this.workbenchModeService.availableModes.map(({ id, label }) => ({ value: id, label })),
      ariaLabel: "Workbench mode",
      presentation: "field",
      contextViewProvider: this.contextViewProvider,
    }));
    select.element.classList.add("zeta-general-setting-control");
    select.element.dataset.configurationKey = WorkbenchConfiguration.mode.key;
    setting.append(this.createSettingCopy("Workbench mode", "Switch the capability assembly for this window. The current window reloads after a change."), select.element);
    this.controls.set(WorkbenchConfiguration.mode.key, select);
    this.own(select.onDidSelect(({ value }) => {
      const mode = this.workbenchModeService.availableModes.find(candidate => candidate.id === value);
      if (mode) void this.switchWorkbenchMode(mode.id);
    }));
    return setting;
  }

  private createSelectSetting<T extends string>(options: { readonly key: IConfigurationKey<T>; readonly label: string; readonly description: string; readonly options: readonly { readonly value: T; readonly label: string }[] }): HTMLElement {
    const setting = h(this.element.ownerDocument, "div");
    setting.className = "zeta-general-setting";
    const select = this.own(new SelectBox(setting, {
      options: options.options,
      ariaLabel: options.label,
      presentation: "field",
      contextViewProvider: this.contextViewProvider,
    }));
    select.element.classList.add("zeta-general-setting-control");
    select.element.dataset.configurationKey = options.key.key;
    setting.append(this.createSettingCopy(options.label, options.description), select.element);
    this.controls.set(options.key.key, select);
    this.own(select.onDidSelect(({ value }) => void this.updateConfiguration(options.key, value as T)));
    return setting;
  }

  private createNumberSetting(key: IConfigurationKey<number>, label: string, description: string, minimum: number, maximum: number): HTMLElement {
    const setting = h(this.element.ownerDocument, "label");
    setting.className = "zeta-general-setting";
    const input = h(this.element.ownerDocument, "input");
    input.className = "zeta-general-setting-control";
    input.type = "number";
    input.min = String(minimum);
    input.max = String(maximum);
    input.step = "1";
    input.dataset.configurationKey = key.key;
    setting.append(this.createSettingCopy(label, description), input);
    this.controls.set(key.key, input);
    this.own(addDisposableListener(input, "change", () => {
      const value = input.valueAsNumber;
      if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
        this.syncControl(key);
        this.showStatus(`${label} must be between ${minimum} and ${maximum}.`, true);
        return;
      }
      void this.updateConfiguration(key, value);
    }));
    return setting;
  }

  private createSettingCopy(label: string, description: string): HTMLElement {
    const copy = h(this.element.ownerDocument, "span");
    copy.className = "zeta-general-setting-copy";
    const title = h(this.element.ownerDocument, "span");
    title.className = "zeta-general-setting-title";
    title.textContent = label;
    const hint = h(this.element.ownerDocument, "span");
    hint.className = "zeta-general-setting-description";
    hint.textContent = description;
    copy.append(title, hint);
    return copy;
  }

  private syncControls(): void {
    const workbenchModeControl = this.controls.get(WorkbenchConfiguration.mode.key);
    if (workbenchModeControl instanceof SelectBox) workbenchModeControl.value = this.workbenchModeService.currentModeId;
    this.syncControl(AccessibilityConfiguration.editorAccessibilitySupport);
    this.syncControl(AccessibilityConfiguration.reduceMotion);
    this.syncControl(AccessibilityConfiguration.reduceTransparency);
    this.syncControl(AccessibilityConfiguration.underlineLinks);
    this.syncControl(HoverConfiguration.delay);
    this.syncControl(HoverConfiguration.reducedDelay);
    this.syncControl(SashConfiguration.size);
    this.syncControl(SashConfiguration.hoverDelay);
  }

  private async switchWorkbenchMode(modeId: WorkbenchModeId): Promise<void> {
    this.setControlsEnabled(false);
    try {
      await this.workbenchModeService.switchMode(modeId);
    } catch (error) {
      this.syncControls();
      this.showStatus(error instanceof Error ? error.message : "Unable to switch Workbench mode.", true);
      this.setControlsEnabled(true);
    }
  }

  private syncControl<T>(key: IConfigurationKey<T>): void {
    const control = this.controls.get(key.key);
    if (!control) return;
    const value = this.configurationService.getValue(key);
    if (control instanceof Toggle) control.checked = value as boolean;
    else if (control instanceof this.element.ownerDocument.defaultView!.HTMLInputElement && control.type === "checkbox") control.checked = value as boolean;
    else control.value = String(value);
  }

  private async updateConfiguration<T>(key: IConfigurationKey<T>, value: T): Promise<void> {
    this.setControlsEnabled(false);
    try {
      await this.configurationService.updateValue(key, value);
      this.showStatus("Setting saved.", false);
    } catch (error) {
      this.syncControl(key);
      this.showStatus(error instanceof Error ? error.message : "Unable to save the setting.", true);
    } finally {
      this.setControlsEnabled(true);
    }
  }

  private setControlsEnabled(enabled: boolean): void {
    for (const control of this.controls.values()) {
      if (control instanceof SelectBox) control.enabled = enabled;
      else if (control instanceof Toggle) control.enabled = enabled;
      else control.disabled = !enabled;
    }
  }

  private showStatus(message: string, error: boolean): void {
    this.status.textContent = message;
    this.status.classList.toggle("is-error", error);
  }
}

function triStateOptions<T extends AccessibilitySupportConfiguration | AccessibilityReductionConfiguration>(): readonly { readonly value: T; readonly label: string }[] {
  return [
    { value: "auto" as T, label: "Auto" },
    { value: "on" as T, label: "On" },
    { value: "off" as T, label: "Off" },
  ];
}

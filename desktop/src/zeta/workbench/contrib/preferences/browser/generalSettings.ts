import "./media/generalSettings.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import type { IContextViewProvider } from "../../../../base/browser/ui/contextview/contextview.js";
import { SelectBox } from "../../../../base/browser/ui/selectbox/selectbox.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { AccessibilityConfiguration, type AccessibilityReductionConfiguration, type AccessibilitySupportConfiguration } from "../../../../platform/accessibility/common/accessibility.js";
import type { IConfigurationKey, IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { HoverConfiguration, MaximumHoverDelay, MinimumHoverDelay } from "../../../../platform/hover/common/hoverService.js";
import { MaximumSashHoverDelay, MaximumSashSize, MinimumSashHoverDelay, MinimumSashSize, SashConfiguration } from "../../sash/common/sash.js";
import { SettingsTree } from "./settingsTree.js";
import { SettingsTreeModel, type SettingsTreeNode } from "./settingsTreeModels.js";

type GeneralControl = HTMLInputElement | SelectBox;

/** Core application preferences that are independent of one editor or feature domain. */
export class GeneralSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly controls = new Map<string, GeneralControl>();
  private readonly status: HTMLParagraphElement;

  constructor(ownerDocument: Document, private readonly configurationService: IConfigurationService, private readonly contextViewProvider: IContextViewProvider) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-general-settings";
    const model = this.own(new SettingsTreeModel<HTMLElement>());
    model.setChildren([
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
    const tree = this.own(new SettingsTree({
      ownerDocument,
      model,
      rootClassName: "zeta-general-settings-tree",
      groupClassName: "zeta-general-settings-group",
      groupDescriptionClassName: "zeta-general-settings-group-description",
      itemsClassName: "zeta-general-settings-list",
      renderItem: (item) => item.value,
    }));
    this.element.append(tree.element);
    this.status = ownerDocument.createElement("p");
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
    const setting = this.element.ownerDocument.createElement("label");
    setting.className = "zeta-general-setting zeta-general-toggle-setting";
    const input = this.element.ownerDocument.createElement("input");
    input.type = "checkbox";
    input.dataset.configurationKey = key.key;
    const copy = this.createSettingCopy(label, description);
    setting.append(copy, input);
    this.controls.set(key.key, input);
    this.own(addDisposableListener(input, "change", () => void this.updateConfiguration(key, input.checked)));
    return setting;
  }

  private createSelectSetting<T extends string>(options: { readonly key: IConfigurationKey<T>; readonly label: string; readonly description: string; readonly options: readonly { readonly value: T; readonly label: string }[] }): HTMLElement {
    const setting = this.element.ownerDocument.createElement("div");
    setting.className = "zeta-general-setting";
    const select = this.own(new SelectBox({
      options: options.options,
      ownerDocument: this.element.ownerDocument,
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
    const setting = this.element.ownerDocument.createElement("label");
    setting.className = "zeta-general-setting";
    const input = this.element.ownerDocument.createElement("input");
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
    const copy = this.element.ownerDocument.createElement("span");
    copy.className = "zeta-general-setting-copy";
    const title = this.element.ownerDocument.createElement("span");
    title.className = "zeta-general-setting-title";
    title.textContent = label;
    const hint = this.element.ownerDocument.createElement("span");
    hint.className = "zeta-general-setting-description";
    hint.textContent = description;
    copy.append(title, hint);
    return copy;
  }

  private syncControls(): void {
    this.syncControl(AccessibilityConfiguration.editorAccessibilitySupport);
    this.syncControl(AccessibilityConfiguration.reduceMotion);
    this.syncControl(AccessibilityConfiguration.reduceTransparency);
    this.syncControl(AccessibilityConfiguration.underlineLinks);
    this.syncControl(HoverConfiguration.delay);
    this.syncControl(HoverConfiguration.reducedDelay);
    this.syncControl(SashConfiguration.size);
    this.syncControl(SashConfiguration.hoverDelay);
  }

  private syncControl<T>(key: IConfigurationKey<T>): void {
    const control = this.controls.get(key.key);
    if (!control) return;
    const value = this.configurationService.getValue(key);
    if (control instanceof this.element.ownerDocument.defaultView!.HTMLInputElement && control.type === "checkbox") control.checked = value as boolean;
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

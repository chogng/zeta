import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IPluginService, PluginCatalogView, PluginPackageView } from "../../../../platform/plugins/common/pluginService.js";
import "./media/connectorSettings.css";

export class PluginSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly rows = this.own(new ResettableDisposableGroup());
  private loadGeneration = 0;

  constructor(private readonly document: Document, private readonly plugins: IPluginService) {
    super();
    this.element = document.createElement("div");
    this.element.className = "zeta-integration-settings";
    this.own(plugins.onDidChange(() => void this.reload()));
    void this.reload();
    this.defer(() => this.element.remove());
  }

  private async reload(): Promise<void> {
    const loadGeneration = ++this.loadGeneration;
    const loading = this.document.createElement("p");
    loading.className = "zeta-settings-message";
    loading.textContent = "Loading plugins…";
    this.element.replaceChildren(loading);
    const catalog = await this.plugins.list().catch((error: unknown) => {
      if (loadGeneration !== this.loadGeneration) return undefined;
      loading.textContent = error instanceof Error ? `Unable to load plugins: ${error.message}` : "Unable to load plugins.";
      return undefined;
    });
    if (!catalog || loadGeneration !== this.loadGeneration) return;
    this.render(catalog);
  }

  private render(catalog: PluginCatalogView): void {
    this.rows.clear();
    if (catalog.packages.length === 0) {
      const empty = this.document.createElement("p");
      empty.className = "zeta-settings-message";
      empty.textContent = "No plugins are installed.";
      this.element.replaceChildren(empty);
      return;
    }
    const fragment = this.document.createDocumentFragment();
    for (const plugin of catalog.packages) fragment.append(this.pluginCard(catalog.revision, plugin));
    this.element.replaceChildren(fragment);
  }

  private pluginCard(revision: number, plugin: PluginPackageView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const heading = this.document.createElement("div");
    heading.className = "zeta-integration-heading";
    const title = this.document.createElement("h4");
    title.textContent = `${plugin.id} · ${plugin.version}`;
    const state = this.document.createElement("span");
    state.className = `zeta-integration-state is-${plugin.effective ? "connected" : "disconnected"}`;
    state.textContent = status(plugin);
    heading.append(title, state);
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    card.append(heading);
    if (!plugin.granted) card.append(this.action("Grant", () => this.plugins.grant(plugin, revision), feedback));
    if (plugin.granted) card.append(this.action("Revoke grant", () => this.plugins.revokeGrant(plugin, revision), feedback));
    if (!plugin.enabled) card.append(this.action("Enable", () => this.plugins.enable(plugin, revision), feedback));
    if (plugin.enabled) card.append(this.action("Disable", () => this.plugins.disable(plugin, revision), feedback));
    if (!plugin.enabled && !plugin.granted) card.append(this.action("Uninstall", () => this.plugins.uninstall(plugin, revision), feedback, true));
    card.append(feedback);
    return card;
  }

  private action(label: string, invoke: () => Promise<void>, feedback: HTMLElement, danger = false): HTMLButtonElement {
    const button = this.document.createElement("button");
    button.type = "button";
    button.className = `zeta-theme-action${danger ? " is-danger" : ""}`;
    button.textContent = label;
    this.rows.add(addDisposableListener(button, "click", () => {
      button.disabled = true;
      feedback.textContent = `${label}…`;
      void invoke().then(() => this.reload()).catch((error: unknown) => {
        button.disabled = false;
        feedback.textContent = error instanceof Error ? `${label} failed: ${error.message}` : `${label} failed.`;
      });
    }));
    return button;
  }
}

function status(plugin: PluginPackageView): string {
  if (plugin.effective) return "Active";
  if (plugin.enabled && !plugin.granted) return "Enabled · grant required";
  if (!plugin.enabled && plugin.granted) return "Granted · disabled";
  return "Installed";
}

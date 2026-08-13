import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import type { ILanguageMarketplaceService, LanguageMarketplaceCatalogView, LanguageMarketplaceEntryView } from "../../../../platform/language/common/languageMarketplaceService.js";
import "./media/connectorSettings.css";

export class LanguageMarketplaceSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly rows = this.own(new ResettableDisposableGroup());
  private loadGeneration = 0;

  constructor(private readonly document: Document, private readonly marketplace: ILanguageMarketplaceService, private readonly dialogs: IDialogService) {
    super();
    this.element = document.createElement("div");
    this.element.className = "zeta-integration-settings";
    void this.reload();
    this.defer(() => this.element.remove());
  }

  private async reload(): Promise<void> {
    const generation = ++this.loadGeneration;
    const loading = this.document.createElement("p");
    loading.className = "zeta-settings-message";
    loading.textContent = "Loading signed language packages…";
    this.element.replaceChildren(loading);
    const catalog = await this.marketplace.list().catch((error: unknown) => {
      if (generation !== this.loadGeneration) return undefined;
      loading.textContent = error instanceof Error ? `Unable to load language packages: ${error.message}` : "Unable to load language packages.";
      return undefined;
    });
    if (!catalog || generation !== this.loadGeneration) return;
    this.render(catalog);
  }

  private render(catalog: LanguageMarketplaceCatalogView): void {
    this.rows.clear();
    if (catalog.entries.length === 0) {
      const empty = this.document.createElement("p");
      empty.className = "zeta-settings-message";
      empty.textContent = "No signed language-server packages are available.";
      this.element.replaceChildren(empty);
      return;
    }
    this.element.replaceChildren(...catalog.entries.map(entry => this.card(catalog, entry)));
  }

  private card(catalog: LanguageMarketplaceCatalogView, entry: LanguageMarketplaceEntryView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const heading = this.document.createElement("div");
    heading.className = "zeta-integration-heading";
    const title = this.document.createElement("h4");
    title.textContent = entry.displayName;
    const state = this.document.createElement("span");
    state.className = `zeta-integration-state is-${entry.active ? "connected" : "disconnected"}`;
    state.textContent = entry.active ? "Active" : "Available";
    heading.append(title, state);
    const source = this.document.createElement("p");
    source.className = "zeta-connector-description";
    source.textContent = `${entry.packageId} · ${entry.version} · ${entry.marketplaceId}`;
    const description = this.document.createElement("p");
    description.className = "zeta-marketplace-description";
    description.textContent = entry.description;
    const languages = this.document.createElement("div");
    languages.className = "zeta-marketplace-capabilities";
    for (const language of entry.languages) {
      const badge = this.document.createElement("span");
      badge.textContent = language;
      languages.append(badge);
    }
    const details = this.document.createElement("p");
    details.className = "zeta-marketplace-details";
    details.textContent = `${entry.serverId} · ${entry.license} · ${entry.fileExtensions.join(", ")}`;
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    card.append(heading, source, description, languages, details);
    if (entry.compatibility.status === "incompatible") {
      const unavailable = this.document.createElement("p");
      unavailable.className = "zeta-integration-state is-unavailable";
      unavailable.textContent = entry.compatibility.reason;
      card.append(unavailable);
    } else if (!entry.active) {
      const install = this.document.createElement("button");
      install.type = "button";
      install.className = "zeta-theme-action";
      install.textContent = "Install and activate";
      this.rows.add(addDisposableListener(install, "click", () => {
        void this.confirmAndInstall(install, feedback, catalog, entry);
      }));
      card.append(install);
    }
    card.append(feedback);
    return card;
  }

  private async confirmAndInstall(button: HTMLButtonElement, feedback: HTMLElement, catalog: LanguageMarketplaceCatalogView, entry: LanguageMarketplaceEntryView): Promise<void> {
    const confirmed = await this.dialogs.confirm({
      title: "Install language support?",
      message: `Install ${entry.displayName} ${entry.version}?`,
      detail: `Zeta will download this exact TUF-signed package (${entry.digest}), verify it, and run ${entry.serverId} with Zeta's shared Node-compatible runtime. Languages: ${entry.languages.join(", ")}.`,
      primaryButton: "Install and activate",
      cancelButton: "Cancel",
    });
    if (!confirmed) return;
    button.disabled = true;
    feedback.textContent = "Downloading and verifying…";
    await this.marketplace.install(entry, catalog.revision).then(() => this.reload()).catch((error: unknown) => {
      button.disabled = false;
      feedback.textContent = error instanceof Error ? `Install failed: ${error.message}` : "Install failed.";
    });
  }
}

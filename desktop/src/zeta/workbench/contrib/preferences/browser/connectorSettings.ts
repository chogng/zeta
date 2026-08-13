import "./media/connectorSettings.css";
import { addDisposableListener } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { ConnectorCatalogView, ConnectorState, ConnectorView, IConnectorService } from "../../../../platform/connectors/common/connectorService.js";

/** Settings-owned projection of Connector catalog and credential actions. */
export class ConnectorSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly rows = this.own(new ResettableDisposableGroup());
  private loadGeneration = 0;

  constructor(private readonly document: Document, private readonly connectors: IConnectorService) {
    super();
    this.element = document.createElement("div");
    this.element.className = "zeta-integration-settings";
    this.own(connectors.onDidChange(() => void this.reload()));
    void this.reload();
    this.defer(() => this.element.remove());
  }

  private async reload(): Promise<void> {
    const loadGeneration = ++this.loadGeneration;
    const loading = this.document.createElement("p");
    loading.className = "zeta-settings-message";
    loading.textContent = "Loading connectors…";
    this.element.replaceChildren(loading);
    const catalog = await this.connectors.list().catch((error: unknown) => {
      if (loadGeneration !== this.loadGeneration) return undefined;
      loading.textContent = error instanceof Error ? `Unable to load connectors: ${error.message}` : "Unable to load connectors.";
      return undefined;
    });
    if (!catalog || loadGeneration !== this.loadGeneration) return;
    this.render(catalog);
  }

  private render(catalog: ConnectorCatalogView): void {
    this.rows.clear();
    if (catalog.connectors.length === 0) {
      const empty = this.document.createElement("p");
      empty.className = "zeta-settings-message";
      empty.textContent = "No active plugin contributes a Connector.";
      this.element.replaceChildren(empty);
      return;
    }
    const fragment = this.document.createDocumentFragment();
    for (const connector of catalog.connectors) fragment.append(this.connectorCard(catalog.generation, connector));
    this.element.replaceChildren(fragment);
  }

  private connectorCard(catalogGeneration: number, connector: ConnectorView): HTMLElement {
    const card = this.document.createElement("section");
    card.className = "zeta-integration-card";
    const heading = this.document.createElement("div");
    heading.className = "zeta-integration-heading";
    const title = this.document.createElement("h4");
    title.textContent = connector.displayName;
    const state = this.document.createElement("span");
    state.className = `zeta-integration-state is-${connector.state.status}`;
    state.textContent = stateLabel(connector.state);
    heading.append(title, state);
    const description = this.document.createElement("p");
    description.className = "zeta-connector-description";
    description.textContent = connector.description;
    const feedback = this.document.createElement("p");
    feedback.className = "zeta-integration-feedback";
    feedback.setAttribute("role", "status");
    card.append(heading, description);
    if (connector.canConnectApiToken) card.append(this.connectForm(catalogGeneration, connector, feedback));
    if (connector.canConnectOAuth) card.append(this.oauthButton(catalogGeneration, connector, feedback));
    if (connector.canDisconnect) card.append(this.disconnectButton(catalogGeneration, connector, feedback));
    card.append(feedback);
    return card;
  }

  private connectForm(catalogGeneration: number, connector: ConnectorView, feedback: HTMLElement): HTMLFormElement {
    const form = this.document.createElement("form");
    form.className = "zeta-connector-connect-form";
    const accountId = input(this.document, "Account ID", "External account identity");
    const accountName = input(this.document, "Account name", "Account display name");
    const token = input(this.document, "API token", "API token", "password");
    token.autocomplete = "off";
    const submit = this.document.createElement("button");
    submit.type = "submit";
    submit.className = "zeta-theme-action";
    submit.textContent = connector.state.status === "reauthorizationRequired" ? "Reconnect" : "Connect";
    form.append(accountId, accountName, token, submit);
    this.rows.add(addDisposableListener(form, "submit", (event: SubmitEvent) => {
      event.preventDefault();
      const values = {
        accountId: accountId.value.trim(),
        accountDisplayName: accountName.value.trim(),
        token: token.value,
      };
      if (!values.accountId || !values.accountDisplayName || !values.token) {
        feedback.textContent = "Account ID, account name, and API token are required.";
        return;
      }
      submit.disabled = true;
      feedback.textContent = "Connecting…";
      void this.connectors.connectApiToken(connector, catalogGeneration, values).then(() => {
        token.value = "";
        feedback.textContent = "Connected.";
        return this.reload();
      }).catch((error: unknown) => {
        token.value = "";
        submit.disabled = false;
        feedback.textContent = error instanceof Error ? `Connection failed: ${error.message}` : "Connection failed.";
      });
    }));
    return form;
  }

  private disconnectButton(catalogGeneration: number, connector: ConnectorView, feedback: HTMLElement): HTMLButtonElement {
    const button = this.document.createElement("button");
    button.type = "button";
    button.className = "zeta-theme-action is-danger";
    button.textContent = "Disconnect";
    this.rows.add(addDisposableListener(button, "click", () => {
      button.disabled = true;
      feedback.textContent = "Disconnecting…";
      void this.connectors.disconnect(connector, catalogGeneration).then(() => {
        feedback.textContent = "Disconnected.";
        return this.reload();
      }).catch((error: unknown) => {
        button.disabled = false;
        feedback.textContent = error instanceof Error ? `Disconnect failed: ${error.message}` : "Disconnect failed.";
      });
    }));
    return button;
  }

  private oauthButton(catalogGeneration: number, connector: ConnectorView, feedback: HTMLElement): HTMLButtonElement {
    const button = this.document.createElement("button");
    button.type = "button";
    button.className = "zeta-theme-action";
    button.textContent = connector.state.status === "reauthorizationRequired" ? "Reconnect with OAuth" : "Connect with OAuth";
    this.rows.add(addDisposableListener(button, "click", () => {
      button.disabled = true;
      feedback.textContent = "Waiting for browser authorization…";
      void this.connectors.connectOAuth(connector, catalogGeneration).then(() => {
        feedback.textContent = "Connected.";
        return this.reload();
      }).catch((error: unknown) => {
        button.disabled = false;
        feedback.textContent = error instanceof Error ? `OAuth connection failed: ${error.message}` : "OAuth connection failed.";
      });
    }));
    return button;
  }
}

function input(document: Document, placeholder: string, ariaLabel: string, type = "text"): HTMLInputElement {
  const element = document.createElement("input");
  element.className = "zeta-settings-text-input";
  element.type = type;
  element.placeholder = placeholder;
  element.setAttribute("aria-label", ariaLabel);
  return element;
}

function stateLabel(state: ConnectorState): string {
  switch (state.status) {
    case "disconnected": return "Not connected";
    case "connecting": return "Connecting";
    case "connected": return `Connected · ${state.account.displayName}`;
    case "unavailable": return `Unavailable · ${state.reason}`;
    case "reauthorizationRequired": return `Reconnect · ${state.account.displayName}`;
  }
}

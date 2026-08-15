import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { DialogSeverity, type IDialogService } from "../../../../platform/dialogs/common/dialogs.js";
import { type IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import { type LanguageServerMessageNotification, type LanguageServerMessageSeverityDto, type LanguageServerProgressNotification } from "../../../../../../generated/app-server/types.js";
import { StatusbarAlignment, type IStatusbarEntry, type IStatusbarEntryAccessor, type IStatusbarService } from "../../statusbar/browser/statusbar.js";
import { type ILanguageServerStatusService, type LanguageServerLogEntry, type LanguageServerProgressState } from "../common/languageServerStatusService.js";

const MAX_LOG_ENTRIES = 2_000;

/** Projects App Server language notifications into Workbench dialogs, logs, and status. */
export class AppServerLanguageServerStatusService extends DisposableOwner implements ILanguageServerStatusService {
  private readonly changeEmitter = this.own(new Emitter<void>());
  private readonly logs: LanguageServerLogEntry[] = [];
  private readonly progress = new Map<string, LanguageServerProgressState>();
  private readonly status: IStatusbarEntryAccessor;
  private nextSequence = 1;
  readonly onDidChange = this.changeEmitter.event;

  constructor(events: IServerEventApi, private readonly dialogs: IDialogService, statusbar: IStatusbarService, private readonly openOutput: () => unknown) {
    super();
    this.status = this.own(statusbar.addEntry(this.statusEntry("Language Servers", "No active language-server operation"), { id: "zeta.status.languageServers", alignment: StatusbarAlignment.Right, priority: 20 }));
    const subscription = events.subscribe(event => {
      if (event.method === "language/serverMessage") this.acceptMessage(event.params);
      if (event.method === "language/serverProgress") this.acceptProgress(event.params);
    });
    this.defer(() => subscription.dispose());
  }

  getLogEntries(): readonly LanguageServerLogEntry[] {
    return Object.freeze([...this.logs]);
  }

  getProgress(): readonly LanguageServerProgressState[] {
    return Object.freeze([...this.progress.values()]);
  }

  clearLog(): void {
    if (this.logs.length === 0) return;
    this.logs.length = 0;
    this.changeEmitter.fire();
  }

  private acceptMessage(message: LanguageServerMessageNotification): void {
    const text = message.message.trim();
    if (!text) return;
    this.logs.push(Object.freeze({ sequence: this.nextSequence++, server: message.server, severity: message.severity, message: text }));
    if (this.logs.length > MAX_LOG_ENTRIES) this.logs.splice(0, this.logs.length - MAX_LOG_ENTRIES);
    this.changeEmitter.fire();
    if (message.show) void this.dialogs.showMessage({ title: message.server, severity: dialogSeverity(message.severity), message: text });
  }

  private acceptProgress(update: LanguageServerProgressNotification): void {
    const key = `${update.server}\0${update.token}`;
    const current = this.progress.get(key);
    if (update.done) {
      this.progress.delete(key);
    } else {
      const title = update.title?.trim() || current?.title || "Language server operation";
      this.progress.set(key, Object.freeze({ server: update.server, token: update.token, title, ...(update.message?.trim() ? { message: update.message.trim() } : current?.message ? { message: current.message } : {}), ...(update.percentage === null ? current?.percentage === undefined ? {} : { percentage: current.percentage } : { percentage: update.percentage }) }));
    }
    this.updateStatus();
    this.changeEmitter.fire();
  }

  private updateStatus(): void {
    const active = [...this.progress.values()];
    if (active.length === 0) {
      this.status.update(this.statusEntry("Language Servers", "No active language-server operation"));
      return;
    }
    const first = active[0]!;
    const percentage = first.percentage === undefined ? "" : ` ${first.percentage}%`;
    const suffix = active.length === 1 ? "" : ` (+${active.length - 1})`;
    this.status.update(this.statusEntry(`${first.title}${percentage}${suffix}`, [first.server, first.message].filter(Boolean).join(": ")));
  }

  private statusEntry(text: string, tooltip: string): IStatusbarEntry {
    return { text, tooltip, run: this.openOutput };
  }
}

function dialogSeverity(severity: LanguageServerMessageSeverityDto): DialogSeverity {
  if (severity === "error") return DialogSeverity.Error;
  if (severity === "warning") return DialogSeverity.Warning;
  return DialogSeverity.Info;
}

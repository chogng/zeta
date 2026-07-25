import { Button } from "../../base/browser/ui/index.js";
import type { ZetaRendererApi } from "../../platform/app-server/common/renderer-api.js";
import { CommandRegistry } from "../../platform/commands/common/command-registry.js";

/** Starts the browser workbench and binds its commands to the initial UI. */
export function startWorkbench(api: ZetaRendererApi, container: Element | null): void {
  const commands = new CommandRegistry();
  commands.register("zeta.startTurn", async () => {
    const thread = await api.thread.start({ idempotencyKey: crypto.randomUUID(), title: "New conversation" });
    await api.turn.start({ idempotencyKey: crypto.randomUUID(), threadId: thread.threadId, input: [{ type: "text", text: "Hello" }] });
  });

  new Button({ label: "Start conversation", onClick: () => commands.execute("zeta.startTurn") }).mount(container ?? document.body);
}

import { CommandRegistry } from "./commands/command-registry.js";

declare global { interface Window { zeta: import("../preload/api.js").ZetaPreloadApi; } }

const commands = new CommandRegistry();
commands.register("zeta.startTurn", async () => {
  const thread = await window.zeta.thread.start({ idempotencyKey: crypto.randomUUID(), title: "New conversation" });
  await window.zeta.turn.start({ idempotencyKey: crypto.randomUUID(), threadId: thread.threadId, input: [{ type: "text", text: "Hello" }] });
});
const button = document.createElement("button");
button.textContent = "Start conversation";
button.onclick = () => void commands.execute("zeta.startTurn");
document.querySelector("#app")?.append(button);

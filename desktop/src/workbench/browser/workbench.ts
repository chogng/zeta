import { Button } from "../../base/browser/ui/index.js";
import { platform } from "../../base/common/platform.js";
import type { ZetaRendererApi } from "../../platform/app-server/common/renderer-api.js";
import { CommandRegistry } from "../../platform/commands/common/command-registry.js";
import { WorkbenchLayout, type SerializableGrid, type WorkbenchPartId } from "./layout.js";
import type { WorkbenchPart } from "./part.js";
import { AuxiliarybarPart, EditorPart, SessionPart, SidebarPart, StatusbarPart, TitlebarPart, ViewPaneContainer, Viewlet } from "./parts/index.js";
import { installWorkbenchStyles } from "./style.js";

/** Starts the browser workbench and binds its commands to the initial UI. */
export function startWorkbench(api: ZetaRendererApi, container: Element | null): void {
  installWorkbenchStyles();
  const workbenchRoot = container ?? document.body;
  workbenchRoot.classList.add("zeta-workbench");
  workbenchRoot.setAttribute("data-platform", platform);
  const commands = new CommandRegistry();
  commands.register("zeta.startTurn", async () => {
    const { session } = await api.session.create({
      commandId: crypto.randomUUID(),
      title: "New conversation",
    });
    const created = await api.session.createThread({
      commandId: crypto.randomUUID(),
      sessionId: session.sessionId,
      expectedSequence: session.sequence,
      title: "Main",
    });
    const { thread } = await api.thread.read({ threadId: created.threadId });
    await api.turn.start({
      commandId: crypto.randomUUID(),
      sessionId: session.sessionId,
      threadId: thread.threadId,
      expectedSequence: thread.sequence,
      input: [{ type: "text", text: "Hello" }],
    });
  });

  const titlebar = new TitlebarPart("Zeta", [{ id: "zeta.startTurn", label: "New conversation", run: () => commands.execute("zeta.startTurn") }]);
  const sidebar = new SidebarPart();
  sidebar.setViewlet(new Viewlet("zeta.sidebar", "Navigation"));
  const session = new SessionPart();
  const editor = new EditorPart();
  editor.setContent(new Button({ label: "Start conversation", onClick: () => commands.execute("zeta.startTurn") }).element);
  const auxiliarybar = new AuxiliarybarPart();
  auxiliarybar.setViewPaneContainer(new ViewPaneContainer("zeta.auxiliary"));
  const statusbar = new StatusbarPart();

  const parts = new Map<WorkbenchPartId, WorkbenchPart>([
    ["titlebar", titlebar],
    ["statusbar", statusbar],
    ["sidebar", sidebar],
    ["session", session],
    ["auxiliarybar", auxiliarybar],
    ["editor", editor],
  ]);
  new WorkbenchLayout(workbenchRoot, parts, defaultWorkbenchGrid).layout();
}

const defaultWorkbenchGrid: SerializableGrid = {
  type: "split",
  orientation: "vertical",
  children: [
    { type: "part", partId: "titlebar", size: "34px" },
    {
      type: "split",
      orientation: "horizontal",
      size: "1fr",
      children: [
        { type: "part", partId: "sidebar", size: "220px" },
        {
          type: "split",
          orientation: "vertical",
          size: "1fr",
          children: [
            { type: "part", partId: "session", size: "28px" },
            { type: "part", partId: "editor", size: "1fr" },
          ],
        },
        { type: "part", partId: "auxiliarybar", size: "220px" },
      ],
    },
    { type: "part", partId: "statusbar", size: "22px" },
  ],
};

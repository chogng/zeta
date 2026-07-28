import { LxIcon } from "../../../../base/common/lxicons.js";
import {
  Keybinding,
  logicalKey,
} from "../../../../base/common/keybindings.js";
import {
  Action2,
  MenuId,
  registerAction2,
} from "../../../../platform/actions/common/actions.js";
import type {
  ServicesAccessor,
} from "../../../../platform/instantiation/common/instantiation.js";
import { IRendererApiService } from "../../../common/services.js";

registerAction2(class StartTurnAction extends Action2 {
  constructor() {
    super({
      id: "zeta.startTurn",
      title: "New conversation",
      tooltip: "Start a new conversation",
      icon: LxIcon.add,
      menu: [
        {
          id: MenuId.TitleBar,
          group: "navigation",
          order: 10,
        },
        {
          id: MenuId.MenubarFileMenu,
          group: "1_new",
          order: 10,
        },
      ],
      keybinding: {
        primary: Keybinding.single(logicalKey("n", {
          primaryKey: true,
        })),
      },
      f1: true,
    });
  }

  override async run(accessor: ServicesAccessor): Promise<void> {
    const api = accessor.get(IRendererApiService);
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
    const { thread } = await api.thread.read({
      threadId: created.threadId,
    });
    await api.turn.start({
      commandId: crypto.randomUUID(),
      sessionId: session.sessionId,
      threadId: thread.threadId,
      expectedSequence: thread.sequence,
      input: [{ type: "text", text: "Hello" }],
    });
  }
});

import assert from "node:assert/strict";
import test from "node:test";
import { IME } from "../../../../../base/common/ime.js";
import {
  Keybinding,
  logicalKey,
  physicalKey,
  resolveKeybinding,
} from "../../../../../base/common/keybindings.js";
import { DisposableStore } from "../../../../../base/common/lifecycle.js";
import { OperatingSystem } from "../../../../../base/common/platform.js";
import {
  getKeybindingLabel,
} from "../../../../../base/common/keybindingLabels.js";
import {
  CommandRegistry,
} from "../../../../../platform/commands/common/commands.js";
import {
  ContextKeyExpr,
  ContextKeyService,
} from "../../../../../platform/contextkey/common/contextkey.js";
import {
  parseContextKeyExpression,
} from "../../../../../platform/contextkey/common/contextKeyExpressionParser.js";
import {
  ServiceCollection,
} from "../../../../../platform/instantiation/common/instantiation.js";
import {
  KeybindingResolveKind,
  KeybindingResolver,
} from "../../../../../platform/keybinding/common/keybindingResolver.js";
import {
  KeybindingRegistry,
  KeybindingWeight,
} from "../../../../../platform/keybinding/common/keybindingsRegistry.js";
import {
  BrowserKeyboardLayoutService,
} from "../../../../../workbench/services/keybinding/browser/keyboardLayoutService.js";
import {
  WorkbenchKeybindingService,
} from "../../../../../workbench/services/keybinding/browser/keybindingService.js";
import {
  CommandService,
} from "../../../../../workbench/services/commands/common/commandService.js";
import {
  KeybindingsResourceContribution,
} from "../../../../../workbench/services/keybinding/browser/keybindingsResourceContribution.js";
import {
  WorkbenchKeybindingsResourceService,
} from "../../../../../workbench/services/keybinding/browser/keybindingsResourceService.js";
import {
  StatusbarAlignment,
  StatusbarService,
} from "../../../../../workbench/services/statusbar/browser/statusbar.js";

test("resolver applies context, weight, and latest-registration precedence", () => {
  using registrations = new DisposableStore();
  const registry = new KeybindingRegistry();
  const contexts = registrations.add(new ContextKeyService());
  const keybinding = Keybinding.single(logicalKey("p", {
    ctrlKey: true,
  }));
  registrations.add(registry.registerKeybindingRule({
    command: "test.low",
    keybinding,
    weight: KeybindingWeight.Builtin,
  }));
  registrations.add(registry.registerKeybindingRule({
    command: "test.disabled",
    keybinding,
    when: ContextKeyExpr.has("test.enabled"),
    weight: KeybindingWeight.User,
  }));
  registrations.add(registry.registerKeybindingRule({
    command: "test.latest",
    keybinding,
  }));
  const resolver = new KeybindingResolver({
    registry,
    resolveKeybinding: (keybinding) =>
      resolveKeybinding(keybinding, OperatingSystem.Windows),
  });
  const event = keyEventData();

  let result = resolver.resolve(contexts, [event]);
  assert.equal(result.kind, KeybindingResolveKind.Command);
  assert.equal(
    result.kind === KeybindingResolveKind.Command
      ? result.command
      : undefined,
    "test.latest",
  );

  contexts.setContext("test.enabled", true);
  result = resolver.resolve(contexts, [event]);
  assert.equal(
    result.kind === KeybindingResolveKind.Command
      ? result.command
      : undefined,
    "test.disabled",
  );
});

test("when expressions preserve boolean precedence and comparisons", () => {
  using contexts = new ContextKeyService();
  const expression = parseContextKeyExpression(
    "editorFocus && (mode == edit || !readOnly)",
  );

  contexts.setContext("editorFocus", true);
  contexts.setContext("mode", "preview");
  contexts.setContext("readOnly", false);
  assert.equal(expression.evaluate(contexts), true);

  contexts.setContext("readOnly", true);
  assert.equal(expression.evaluate(contexts), false);
  contexts.setContext("mode", "edit");
  assert.equal(expression.evaluate(contexts), true);
  assert.throws(
    () => parseContextKeyExpression("editorFocus &&"),
    /Expected/,
  );
});

test("browser service executes chords and restores IME state", async () => {
  using registrations = new DisposableStore();
  const registry = new KeybindingRegistry();
  const commands = new CommandRegistry();
  const contexts = registrations.add(new ContextKeyService());
  let executions = 0;
  const executed = new Promise<void>((resolve) => {
    registrations.add(commands.register("test.chord", () => {
      executions += 1;
      resolve();
    }));
  });
  registrations.add(registry.registerKeybindingRule({
    command: "test.chord",
    keybinding: Keybinding.chord(
      physicalKey("KeyK", { ctrlKey: true }),
      physicalKey("KeyC", { ctrlKey: true }),
    ),
  }));
  const keyboardLayout = registrations.add(
    new BrowserKeyboardLayoutService({
      navigator: fakeNavigator(),
      operatingSystem: OperatingSystem.Windows,
    }),
  );
  const statusbar = registrations.add(new StatusbarService());
  const service = registrations.add(new WorkbenchKeybindingService({
    ownerDocument: new EventTarget() as Document,
    commandService: new CommandService(new ServiceCollection(), commands),
    contextKeyService: contexts,
    keyboardLayoutService: keyboardLayout,
    statusbarService: statusbar,
    registry,
  }));

  IME.enable();
  const first = keyboardEvent({ code: "KeyK", key: "k" });
  assert.equal(service.dispatchEvent(first.event), true);
  assert.equal(first.prevented, true);
  assert.equal(IME.enabled, false);
  assert.equal(service.inChordMode, true);
  assert.equal(contexts.getValue("keybinding.inChordMode"), true);
  assert.match(
    statusbar.getEntries(StatusbarAlignment.Left)[0].entry.text,
    /Waiting for another key/,
  );

  const second = keyboardEvent({ code: "KeyC", key: "c" });
  assert.equal(service.dispatchEvent(second.event), true);
  await executed;
  assert.equal(executions, 1);
  assert.equal(IME.enabled, true);
  assert.equal(service.inChordMode, false);
  assert.equal(contexts.getValue("keybinding.inChordMode"), false);
  assert.deepEqual(statusbar.getEntries(StatusbarAlignment.Left), []);
  assert.equal(
    getKeybindingLabel(service.resolveUserBinding("ctrl+k")!),
    "Ctrl+K",
  );
});

test("browser service dispatches Ctrl+Shift+P with a shifted key value", async () => {
  using registrations = new DisposableStore();
  const registry = new KeybindingRegistry();
  const commands = new CommandRegistry();
  const contexts = registrations.add(new ContextKeyService());
  const commandId = "workbench.action.showCommands";
  const executed = new Promise<void>((resolve) => {
    registrations.add(commands.register(commandId, () => resolve()));
  });
  registrations.add(registry.registerKeybindingRule({
    command: commandId,
    keybinding: Keybinding.single(logicalKey("p", {
      ctrlKey: true,
      shiftKey: true,
    })),
  }));
  const keyboardLayout = registrations.add(
    new BrowserKeyboardLayoutService({
      navigator: fakeNavigator(),
      operatingSystem: OperatingSystem.Windows,
    }),
  );
  const service = registrations.add(new WorkbenchKeybindingService({
    ownerDocument: new EventTarget() as Document,
    commandService: new CommandService(new ServiceCollection(), commands),
    contextKeyService: contexts,
    keyboardLayoutService: keyboardLayout,
    registry,
  }));
  const shortcut = keyboardEvent({
    code: "KeyP",
    key: "P",
    shiftKey: true,
  });

  assert.equal(service.dispatchEvent(shortcut.event), true);
  await executed;
  assert.equal(shortcut.prevented, true);
});

test("browser keyboard layouts provide physical key labels", async () => {
  using service = new BrowserKeyboardLayoutService({
    navigator: fakeNavigator(new Map([["KeyY", "z"]])),
    operatingSystem: OperatingSystem.Windows,
  });
  await service.refreshKeyboardLayout();

  const resolved = service.getKeyboardMapper().resolveKeybinding(
    Keybinding.single(physicalKey("KeyY", { ctrlKey: true })),
  );
  assert.equal(getKeybindingLabel(resolved), "Ctrl+Z");
  assert.equal(service.getCurrentKeyboardLayout().source, "browser");
});

test("keybindings resource applies conditions, arguments, OS keys, and blockers", async () => {
  using registrations = new DisposableStore();
  const registry = new KeybindingRegistry();
  const contexts = registrations.add(new ContextKeyService());
  registrations.add(registry.registerKeybindingRule({
    command: "test.builtin",
    keybinding: Keybinding.single(logicalKey("p", {
      ctrlKey: true,
    })),
    weight: KeybindingWeight.Builtin,
  }));
  const keybindingsResource = registrations.add(
    new WorkbenchKeybindingsResourceService(),
  );
  registrations.add(new KeybindingsResourceContribution({
    service: keybindingsResource,
    registry,
    operatingSystem: "windows",
  }));
  const resolver = new KeybindingResolver({
    registry,
    resolveKeybinding: (keybinding) =>
      resolveKeybinding(keybinding, OperatingSystem.Windows),
  });

  await keybindingsResource.updateKeybindings([{
    key: "ctrl+q",
    win: "ctrl+p",
    command: "test.user",
    when: "test.enabled && mode == edit",
    args: { source: "user" },
  }]);
  let result = resolver.resolve(contexts, [keyEventData()]);
  assert.equal(result.kind, KeybindingResolveKind.Command);
  assert.equal(
    result.kind === KeybindingResolveKind.Command
      ? result.command
      : undefined,
    "test.builtin",
  );

  contexts.setContext("test.enabled", true);
  contexts.setContext("mode", "edit");
  result = resolver.resolve(contexts, [keyEventData()]);
  assert.equal(result.kind, KeybindingResolveKind.Command);
  assert.equal(
    result.kind === KeybindingResolveKind.Command
      ? result.command
      : undefined,
    "test.user",
  );
  assert.deepEqual(
    result.kind === KeybindingResolveKind.Command
      ? result.args
      : undefined,
    [{ source: "user" }],
  );
  assert.equal(resolver.lookupKeybinding("test.builtin", contexts), undefined);

  await keybindingsResource.updateKeybindings([{
    key: "ctrl+p",
    command: null,
  }]);
  result = resolver.resolve(contexts, [keyEventData()]);
  assert.equal(result.kind, KeybindingResolveKind.Blocked);
  assert.equal(resolver.lookupKeybinding("test.user", contexts), undefined);
});

function keyEventData() {
  return {
    key: "p",
    code: "KeyP",
    ctrlKey: true,
    shiftKey: false,
    altKey: false,
    metaKey: false,
  } as const;
}

function keyboardEvent(
  overrides: Partial<KeyboardEvent> = {},
): { event: KeyboardEvent; readonly prevented: boolean } {
  let prevented = false;
  const event = {
    key: "p",
    code: "KeyP",
    ctrlKey: true,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    repeat: false,
    isComposing: false,
    target: null,
    composedPath: () => [],
    getModifierState: () => false,
    preventDefault: () => {
      prevented = true;
    },
    stopPropagation: () => {},
    stopImmediatePropagation: () => {},
    ...overrides,
  } as KeyboardEvent;
  return {
    event,
    get prevented() {
      return prevented;
    },
  };
}

function fakeNavigator(
  layout?: ReadonlyMap<string, string>,
): Navigator {
  const keyboard = layout
    ? {
      async getLayoutMap() {
        return layout;
      },
    }
    : undefined;
  return {
    language: "en-US",
    keyboard,
  } as unknown as Navigator;
}

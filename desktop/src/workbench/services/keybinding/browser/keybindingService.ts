import { addDisposableListener } from "../../../../base/browser/dom.js";
import {
  isModifierKey,
  StandardKeyboardEvent,
} from "../../../../base/browser/keyboardEvent.js";
import { Emitter } from "../../../../base/common/event.js";
import { IME } from "../../../../base/common/ime.js";
import {
  getKeybindingLabel,
} from "../../../../base/common/keybindingLabels.js";
import { parseKeybinding } from "../../../../base/common/keybindingParser.js";
import {
  type Keybinding,
  type KeybindingEvent,
  ResolvedKeybinding,
} from "../../../../base/common/keybindings.js";
import {
  DisposableOwner,
  DisposableSlot,
  type IDisposable,
  toDisposable,
} from "../../../../base/common/lifecycle.js";
import type {
  CommandId,
  ICommandService,
} from "../../../../platform/commands/common/command-registry.js";
import {
  type Context,
  type IContextKey,
  type IContextKeyService,
  RawContextKey,
} from "../../../../platform/contextkey/common/contextkey.js";
import type {
  IKeybindingService,
} from "../../../../platform/keybinding/common/keybinding.js";
import {
  KeybindingResolveKind,
  KeybindingResolver,
} from "../../../../platform/keybinding/common/keybindingResolver.js";
import {
  type KeybindingRegistry,
  KeybindingsRegistry,
} from "../../../../platform/keybinding/common/keybindingsRegistry.js";
import type {
  IKeyboardLayoutService,
} from "../../../../platform/keyboardLayout/common/keyboardLayout.js";
import type {
  IStatusbarEntryAccessor,
  IStatusbarService,
} from "../../statusbar/browser/statusbar.js";
import { StatusbarAlignment } from "../../statusbar/browser/statusbar.js";

export const KeybindingContextKeys = {
  inChordMode: new RawContextKey<boolean>(
    "keybinding.inChordMode",
    false,
  ),
  isComposing: new RawContextKey<boolean>(
    "keybinding.isComposing",
    false,
  ),
};

export interface WorkbenchKeybindingServiceOptions {
  readonly ownerDocument: Document;
  readonly commandService: ICommandService;
  readonly contextKeyService: IContextKeyService;
  readonly keyboardLayoutService: IKeyboardLayoutService;
  readonly statusbarService?: IStatusbarService;
  readonly registry?: KeybindingRegistry;
  readonly chordTimeoutMs?: number;
  readonly onCommandError?: (error: unknown, command: CommandId) => void;
}

/**
 * Window-scoped product service that combines keybinding contributions,
 * keyboard layout mapping, ContextKey scopes, command execution, and browser
 * event lifecycle.
 */
export class WorkbenchKeybindingService
  extends DisposableOwner
  implements IKeybindingService {
  readonly #ownerDocument: Document;
  readonly #commandService: ICommandService;
  readonly #contextKeyService: IContextKeyService;
  readonly #keyboardLayoutService: IKeyboardLayoutService;
  readonly #statusbarService: IStatusbarService | undefined;
  readonly #resolver: KeybindingResolver;
  readonly #chordTimeoutMs: number;
  readonly #onCommandError: (error: unknown, command: CommandId) => void;
  readonly #onDidUpdateKeybindings = this.own(new Emitter<void>());
  readonly #chordTimeout = this.own(new DisposableSlot<IDisposable>());
  readonly #chordStatus = this.own(
    new DisposableSlot<IStatusbarEntryAccessor>(),
  );
  readonly #inChordModeKey: IContextKey<boolean>;
  readonly #isComposingKey: IContextKey<boolean>;
  #currentEvents: KeybindingEvent[] = [];
  #disabledIme = false;

  readonly onDidUpdateKeybindings = this.#onDidUpdateKeybindings.event;

  constructor(options: WorkbenchKeybindingServiceOptions) {
    super();
    this.#ownerDocument = options.ownerDocument;
    this.#commandService = options.commandService;
    this.#contextKeyService = options.contextKeyService;
    this.#keyboardLayoutService = options.keyboardLayoutService;
    this.#statusbarService = options.statusbarService;
    this.#resolver = new KeybindingResolver({
      registry: options.registry ?? KeybindingsRegistry,
      resolveKeybinding: (keybinding) => this.resolveKeybinding(keybinding),
    });
    this.#chordTimeoutMs = options.chordTimeoutMs ?? 5_000;
    this.#onCommandError = options.onCommandError ??
      ((error, command) => {
        console.error(`Keybinding command failed: ${command}`, error);
      });
    this.#inChordModeKey = KeybindingContextKeys.inChordMode.bindTo(
      this.#contextKeyService,
    );
    this.#isComposingKey = KeybindingContextKeys.isComposing.bindTo(
      this.#contextKeyService,
    );

    this.defer(() => {
      this.#leaveChordMode();
      this.#isComposingKey.reset();
    });
    this.own(this.#resolver.onDidChangeKeybindings(() => {
      this.#onDidUpdateKeybindings.fire();
    }));
    this.own(this.#keyboardLayoutService.onDidChangeKeyboardLayout(() => {
      this.#leaveChordMode();
      this.#onDidUpdateKeybindings.fire();
    }));
    this.own(addDisposableListener(
      this.#ownerDocument,
      "keydown",
      (event: KeyboardEvent) => this.dispatchEvent(event),
      true,
    ));
    this.own(addDisposableListener(
      this.#ownerDocument,
      "compositionstart",
      () => {
        this.#isComposingKey.set(true);
        this.#leaveChordMode();
      },
      true,
    ));
    this.own(addDisposableListener(
      this.#ownerDocument,
      "compositionend",
      () => this.#isComposingKey.set(false),
      true,
    ));
    const targetWindow = this.#ownerDocument.defaultView;
    if (targetWindow) {
      this.own(addDisposableListener(
        targetWindow,
        "blur",
        () => this.#leaveChordMode(),
      ));
    }
  }

  get inChordMode(): boolean {
    return this.#currentEvents.length > 0;
  }

  resolveKeybinding(keybinding: Keybinding): ResolvedKeybinding {
    return this.#keyboardLayoutService
      .getKeyboardMapper()
      .resolveKeybinding(keybinding);
  }

  resolveUserBinding(
    userBinding: string,
  ): ResolvedKeybinding | undefined {
    const keybinding = parseKeybinding(userBinding);
    return keybinding ? this.resolveKeybinding(keybinding) : undefined;
  }

  lookupKeybindings(
    command: CommandId,
    context: Context = this.#contextKeyService,
  ): readonly ResolvedKeybinding[] {
    return this.#resolver.lookupKeybindings(command, context);
  }

  lookupKeybinding(
    command: CommandId,
    context: Context = this.#contextKeyService,
  ): ResolvedKeybinding | undefined {
    return this.#resolver.lookupKeybinding(command, context);
  }

  /**
   * Dispatches one native event and returns whether a keybinding consumed it.
   */
  dispatchEvent(browserEvent: KeyboardEvent): boolean {
    const event = new StandardKeyboardEvent(browserEvent);
    if (
      event.isComposing ||
      event.altGraphKey ||
      event.key === "Process" ||
      isModifierKey(browserEvent)
    ) {
      return false;
    }

    const nextEvent: KeybindingEvent = {
      key: event.key,
      code: event.code,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
    };
    this.#keyboardLayoutService.validateCurrentKeyboardMapping(nextEvent);
    const events = [...this.#currentEvents, nextEvent];
    const target = keyboardEventTarget(browserEvent);
    const context = this.#contextKeyService.getContext(target);
    const result = this.#resolver.resolve(context, events);

    switch (result.kind) {
      case KeybindingResolveKind.NoMatch:
        if (!this.inChordMode) return false;
        this.#leaveChordMode();
        event.stop();
        return true;

      case KeybindingResolveKind.MoreChordsNeeded:
        this.#currentEvents = events;
        this.#enterChordMode(result.keybinding);
        event.stop();
        return true;

      case KeybindingResolveKind.Command:
        this.#leaveChordMode();
        event.stop();
        void this.#commandService
          .executeCommand(result.command, ...result.args)
          .catch((error: unknown) =>
            this.#onCommandError(error, result.command)
          );
        return true;

      case KeybindingResolveKind.Blocked:
        this.#leaveChordMode();
        event.stop();
        return true;
    }
  }

  #enterChordMode(keybinding: ResolvedKeybinding): void {
    this.#inChordModeKey.set(true);
    if (IME.enabled) {
      IME.disable();
      this.#disabledIme = true;
    }
    const handle = globalThis.setTimeout(
      () => this.#leaveChordMode(),
      this.#chordTimeoutMs,
    );
    this.#chordTimeout.replace(toDisposable(() =>
      globalThis.clearTimeout(handle)
    ));

    if (this.#statusbarService) {
      const prefix = new ResolvedKeybinding(
        keybinding.chords.slice(0, this.#currentEvents.length),
        keybinding.operatingSystem,
      );
      const label = getKeybindingLabel(prefix);
      this.#chordStatus.clear();
      this.#chordStatus.replace(this.#statusbarService.addEntry(
        {
          text: `${label} was pressed. Waiting for another key…`,
          ariaLabel: `${label} was pressed. Waiting for another key`,
        },
        {
          id: "zeta.keybinding.chord",
          alignment: StatusbarAlignment.Left,
          priority: 10_000,
        },
      ));
    }
  }

  #leaveChordMode(): void {
    this.#chordTimeout.clear();
    this.#chordStatus.clear();
    this.#currentEvents = [];
    this.#inChordModeKey.reset();
    if (this.#disabledIme) {
      this.#disabledIme = false;
      IME.enable();
    }
  }
}

function keyboardEventTarget(event: KeyboardEvent): Node | null {
  const first = event.composedPath?.()[0];
  return isNodeLike(first)
    ? first
    : isNodeLike(event.target) ? event.target : null;
}

function isNodeLike(value: unknown): value is Node {
  return typeof value === "object" &&
    value !== null &&
    "nodeType" in value;
}

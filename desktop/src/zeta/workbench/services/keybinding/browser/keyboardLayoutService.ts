import { Emitter } from "../../../../base/common/event.js";
import type {
  Keybinding,
  KeybindingEvent,
  ResolvedKeybinding,
} from "../../../../base/common/keybindings.js";
import {
  resolveKeybinding,
} from "../../../../base/common/keybindings.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import {
  operatingSystem,
  type OperatingSystem,
} from "../../../../base/common/platform.js";
import type {
  IKeyboardLayoutInfo,
  IKeyboardLayoutService,
  IKeyboardMapper,
} from "../../../../platform/keyboardLayout/common/keyboardLayout.js";

interface KeyboardLayoutMapLike extends Iterable<readonly [string, string]> {
  get(code: string): string | undefined;
}

interface NavigatorKeyboardLike {
  getLayoutMap(): Promise<KeyboardLayoutMapLike>;
}

type NavigatorWithKeyboard = Navigator & {
  readonly keyboard?: NavigatorKeyboardLike;
};

export interface BrowserKeyboardLayoutServiceOptions {
  readonly navigator: Navigator;
  readonly operatingSystem?: OperatingSystem;
}

/**
 * Uses the browser Keyboard Map capability when available and falls back to
 * stable `KeyboardEvent.code` labels when the capability is unavailable.
 */
export class BrowserKeyboardLayoutService
  extends DisposableOwner
  implements IKeyboardLayoutService {
  readonly #onDidChangeKeyboardLayout = this.own(new Emitter<void>());
  readonly #navigator: NavigatorWithKeyboard;
  readonly #operatingSystem: OperatingSystem;
  #physicalKeyLabels = new Map<string, string>();
  #mapper: IKeyboardMapper;
  #refreshing: Promise<void> | undefined;
  #disposed = false;

  readonly onDidChangeKeyboardLayout =
    this.#onDidChangeKeyboardLayout.event;

  constructor(options: BrowserKeyboardLayoutServiceOptions) {
    super();
    this.#navigator = options.navigator as NavigatorWithKeyboard;
    this.#operatingSystem = options.operatingSystem ?? operatingSystem;
    this.#mapper = this.#createMapper();
    this.defer(() => {
      this.#disposed = true;
      this.#physicalKeyLabels.clear();
    });
    void this.refreshKeyboardLayout();
  }

  getCurrentKeyboardLayout(): IKeyboardLayoutInfo {
    const browserMapping = this.#physicalKeyLabels.size > 0;
    const language = this.#navigator.language || "unknown";
    return {
      id: browserMapping ? `browser.${language}` : "fallback",
      label: browserMapping ? language : "Fallback keyboard layout",
      source: browserMapping ? "browser" : "fallback",
    };
  }

  getKeyboardMapper(): IKeyboardMapper {
    return this.#mapper;
  }

  validateCurrentKeyboardMapping(event: KeybindingEvent): void {
    if (
      !this.#navigator.keyboard ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey ||
      event.metaKey ||
      event.key.length !== 1
    ) {
      return;
    }
    const expected = this.#physicalKeyLabels.get(event.code);
    if (
      expected === undefined ||
      expected.toLocaleLowerCase("en-US") !==
        event.key.toLocaleLowerCase("en-US")
    ) {
      void this.refreshKeyboardLayout();
    }
  }

  refreshKeyboardLayout(): Promise<void> {
    if (!this.#navigator.keyboard || this.#disposed) {
      return Promise.resolve();
    }
    if (this.#refreshing) return this.#refreshing;

    const refreshing = this.#readKeyboardLayout()
      .finally(() => {
        if (this.#refreshing === refreshing) this.#refreshing = undefined;
      });
    this.#refreshing = refreshing;
    return refreshing;
  }

  async #readKeyboardLayout(): Promise<void> {
    try {
      const layoutMap = await this.#navigator.keyboard!.getLayoutMap();
      if (this.#disposed) return;
      const nextLabels = new Map<string, string>();
      for (const [code, label] of layoutMap) {
        if (code && label) nextLabels.set(code, label);
      }
      if (mapsEqual(this.#physicalKeyLabels, nextLabels)) return;
      this.#physicalKeyLabels = nextLabels;
      this.#mapper = this.#createMapper();
      this.#onDidChangeKeyboardLayout.fire();
    } catch {
      // Browsers may expose the API but deny it without focus or permission.
      // The fallback mapper remains valid and can be refreshed on a later key.
    }
  }

  #createMapper(): IKeyboardMapper {
    const labels = new Map(this.#physicalKeyLabels);
    const targetOperatingSystem = this.#operatingSystem;
    return {
      resolveKeybinding(keybinding: Keybinding): ResolvedKeybinding {
        return resolveKeybinding(
          keybinding,
          targetOperatingSystem,
          labels,
        );
      },
    };
  }
}

function mapsEqual(
  first: ReadonlyMap<string, string>,
  second: ReadonlyMap<string, string>,
): boolean {
  if (first.size !== second.size) return false;
  for (const [key, value] of first) {
    if (second.get(key) !== value) return false;
  }
  return true;
}

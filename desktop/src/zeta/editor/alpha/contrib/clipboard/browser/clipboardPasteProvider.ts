/** Immutable textual snapshot captured synchronously from one native clipboard event. */
export interface ClipboardTextTransfer {
  readonly types: readonly string[];
  getText(type: string): string;
}

/** Extends Alpha paste with one declared clipboard representation. */
export interface ClipboardPasteProvider {
  readonly id: string;
  readonly mimeTypes: readonly string[];
  providePaste(transfer: ClipboardTextTransfer): string | undefined | PromiseLike<string | undefined>;
}

/** Creates a transferable-safe snapshot before a clipboard event returns to the browser. */
export function captureAlphaClipboardTextTransfer(clipboardData: DataTransfer): ClipboardTextTransfer {
  const values = new Map<string, string>();
  const types = [...clipboardData.types].filter(type => typeof type === "string" && type.length > 0);
  for (const type of types) {
    try {
      values.set(type, clipboardData.getData(type));
    } catch {
      // Browsers may deny individual representations while allowing the event.
    }
  }
  return Object.freeze({
    types: Object.freeze([...values.keys()]),
    getText: (type: string): string => values.get(type) ?? "",
  });
}

/** Validates and retains providers in the caller's deterministic precedence order. */
export function normalizeAlphaClipboardPasteProviders(providers: readonly ClipboardPasteProvider[] | undefined): readonly ClipboardPasteProvider[] {
  if (providers === undefined) return Object.freeze([]);
  if (!Array.isArray(providers)) throw new TypeError("Alpha clipboard paste providers must be an array");
  const ids = new Set<string>();
  return Object.freeze(providers.map((provider: ClipboardPasteProvider) => {
    if (!provider || typeof provider !== "object" || typeof provider.id !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(provider.id)) {
      throw new TypeError("Alpha clipboard paste provider requires a stable ID");
    }
    if (ids.has(provider.id)) throw new RangeError(`Duplicate Alpha clipboard paste provider '${provider.id}'`);
    ids.add(provider.id);
    if (!Array.isArray(provider.mimeTypes) || provider.mimeTypes.length === 0 || provider.mimeTypes.some((type: string) => typeof type !== "string" || type.length === 0)) {
      throw new TypeError(`Alpha clipboard paste provider '${provider.id}' requires MIME types`);
    }
    if (new Set(provider.mimeTypes).size !== provider.mimeTypes.length || typeof provider.providePaste !== "function") {
      throw new TypeError(`Alpha clipboard paste provider '${provider.id}' is invalid`);
    }
    return Object.freeze({
      id: provider.id,
      mimeTypes: Object.freeze([...provider.mimeTypes]),
      providePaste: provider.providePaste.bind(provider),
    });
  }));
}

/** Resolves the first matching provider that yields text without exposing native DataTransfer. */
export async function provideAlphaClipboardPaste(providers: readonly ClipboardPasteProvider[], transfer: ClipboardTextTransfer): Promise<string | undefined> {
  for (const provider of providers) {
    if (!provider.mimeTypes.some(type => transfer.types.includes(type))) continue;
    const text = await provider.providePaste(transfer);
    if (text !== undefined) {
      if (typeof text !== "string") throw new TypeError(`Alpha clipboard paste provider '${provider.id}' must return text or undefined`);
      return text;
    }
  }
  return undefined;
}

/** Pastes non-comment URI-list entries in their stable source order. */
export const UriListPasteProvider: ClipboardPasteProvider = Object.freeze({
  id: "alpha.uri-list",
  mimeTypes: Object.freeze(["text/uri-list"]),
  providePaste: (transfer: ClipboardTextTransfer): string | undefined => {
    const values = transfer.getText("text/uri-list").split(/\r?\n/).map((value: string) => value.trim()).filter((value: string) => value.length > 0 && !value.startsWith("#"));
    return values.length > 0 ? values.join("\n") : undefined;
  },
});

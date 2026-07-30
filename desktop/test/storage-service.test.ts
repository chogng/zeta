import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { DisposableStore } from "../src/zeta/base/common/lifecycle.js";
import { StorageScope, StorageTarget, WillSaveStateReason } from "../src/zeta/platform/storage/common/storage.js";
import { BrowserStorageService } from "../src/zeta/workbench/services/storage/browser/storageService.js";

test("Browser storage persists scoped values and target metadata", () => {
  const dom = new JSDOM("<!doctype html><body></body>", {
    url: "https://zeta.test",
  });
  const first = new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-a",
    backend: dom.window.localStorage,
    flushInterval: 0,
  });
  first.store("shared", "application", StorageScope.APPLICATION, StorageTarget.USER);
  first.store("shared", 42, StorageScope.PROFILE, StorageTarget.MACHINE);
  first.store("shared", true, StorageScope.WORKSPACE, StorageTarget.MACHINE);

  assert.equal(first.get("shared", StorageScope.APPLICATION), "application");
  assert.equal(first.getNumber("shared", StorageScope.PROFILE), 42);
  assert.equal(first.getBoolean("shared", StorageScope.WORKSPACE), true);
  assert.deepEqual(first.keys(StorageScope.APPLICATION, StorageTarget.USER), ["shared"]);
  assert.deepEqual(first.keys(StorageScope.PROFILE, StorageTarget.MACHINE), ["shared"]);
  first.dispose();

  const restored = new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-a",
    backend: dom.window.localStorage,
    flushInterval: 0,
  });
  assert.equal(restored.get("shared", StorageScope.APPLICATION), "application");
  assert.equal(restored.getNumber("shared", StorageScope.PROFILE), 42);
  assert.equal(restored.getBoolean("shared", StorageScope.WORKSPACE), true);
  restored.dispose();
  dom.window.close();
});

test("Browser storage isolates workspaces while retaining profile state", () => {
  const dom = new JSDOM("<!doctype html><body></body>", {
    url: "https://zeta.test",
  });
  const first = new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-a",
    backend: dom.window.localStorage,
    flushInterval: 0,
  });
  first.store("size", 260, StorageScope.PROFILE, StorageTarget.MACHINE);
  first.store("visible", false, StorageScope.WORKSPACE, StorageTarget.MACHINE);
  first.dispose();

  const second = new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-b",
    backend: dom.window.localStorage,
    flushInterval: 0,
  });
  assert.equal(second.getNumber("size", StorageScope.PROFILE), 260);
  assert.equal(second.getBoolean("visible", StorageScope.WORKSPACE), undefined);
  second.dispose();
  dom.window.close();
});

test("Browser storage emits changes and will-save lifecycle events", async () => {
  const dom = new JSDOM("<!doctype html><body></body>", {
    url: "https://zeta.test",
  });
  const disposables = new DisposableStore();
  const storage = disposables.add(new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-a",
    backend: dom.window.localStorage,
    flushInterval: 0,
  }));
  const changes: string[] = [];
  const saves: WillSaveStateReason[] = [];
  disposables.add(storage.onDidChangeValue(({ key }) => changes.push(key)));
  disposables.add(storage.onWillSaveState(({ reason }) => saves.push(reason)));

  storage.store("one", 1, StorageScope.PROFILE, StorageTarget.MACHINE);
  storage.store("one", 1, StorageScope.PROFILE, StorageTarget.MACHINE);
  storage.remove("one", StorageScope.PROFILE);
  await storage.flush(WillSaveStateReason.SHUTDOWN);

  assert.deepEqual(changes, ["one", "one"]);
  assert.deepEqual(saves, [WillSaveStateReason.SHUTDOWN]);
  dom.window.dispatchEvent(new dom.window.Event("pagehide"));
  assert.deepEqual(saves, [
    WillSaveStateReason.SHUTDOWN,
    WillSaveStateReason.SHUTDOWN,
  ]);

  disposables.dispose();
  dom.window.close();
});

test("Browser storage projects external document changes", () => {
  const dom = new JSDOM("<!doctype html><body></body>", {
    url: "https://zeta.test",
  });
  const disposables = new DisposableStore();
  const storage = disposables.add(new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-a",
    backend: dom.window.localStorage,
    flushInterval: 0,
  }));
  storage.store("size", 260, StorageScope.PROFILE, StorageTarget.MACHINE);
  const externalChanges: string[] = [];
  disposables.add(storage.onDidChangeValue((event) => {
    if (event.external) externalChanges.push(event.key);
  }));
  const storageKey = [...Array(dom.window.localStorage.length).keys()]
    .map((index) => dom.window.localStorage.key(index))
    .find((key) => key?.includes(".profile."));
  assert.ok(storageKey);
  const document = JSON.parse(dom.window.localStorage.getItem(storageKey)!) as {
    entries: Record<string, { value: string; target: string }>;
  };
  document.entries.size!.value = "310";
  const newValue = JSON.stringify(document);
  dom.window.localStorage.setItem(storageKey, newValue);
  dom.window.dispatchEvent(new dom.window.StorageEvent("storage", {
    key: storageKey,
    newValue,
    storageArea: dom.window.localStorage,
  }));

  assert.equal(storage.getNumber("size", StorageScope.PROFILE), 310);
  assert.deepEqual(externalChanges, ["size"]);

  disposables.dispose();
  dom.window.close();
});

test("Browser storage reports malformed persisted documents and falls back", () => {
  const dom = new JSDOM("<!doctype html><body></body>", {
    url: "https://zeta.test",
  });
  dom.window.localStorage.setItem(
    "zeta.code.storage.profile.default",
    JSON.stringify({ version: 99 }),
  );
  const errors: unknown[] = [];
  const storage = new BrowserStorageService({
    ownerWindow: dom.window as unknown as Window,
    applicationId: "code",
    workspaceId: "workspace-a",
    backend: dom.window.localStorage,
    flushInterval: 0,
    onError: (error) => errors.push(error),
  });

  assert.equal(storage.get("missing", StorageScope.PROFILE), undefined);
  assert.equal(errors.length, 1);

  storage.dispose();
  dom.window.close();
});

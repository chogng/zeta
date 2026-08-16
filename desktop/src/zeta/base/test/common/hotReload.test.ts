import assert from "node:assert/strict";
import test from "node:test";

import { enableHotReload, isHotReloadEnabled, registerHotReloadableClass } from "../../common/hotReload.js";

test("hot reload patches existing class instances and removes stale methods", () => {
  class Initial {
    value(): string { return "initial"; }
    stale(): string { return "stale"; }
  }
  class Replacement {
    value(): string { return "replacement"; }
    added(): string { return "added"; }
  }

  enableHotReload();
  assert.equal(isHotReloadEnabled(), true);
  assert.equal(registerHotReloadableClass("test/hot-class", Initial), "registered");
  const instance = new Initial() as Initial & { added(): string; stale?: () => string };

  assert.equal(registerHotReloadableClass("test/hot-class", Replacement), "patched");
  assert.equal(instance.value(), "replacement");
  assert.equal(instance.added(), "added");
  assert.equal(instance.stale, undefined);
  assert.equal(registerHotReloadableClass("test/hot-class", Initial), "unchanged");
});

test("hot reload rejects a replacement with a different superclass", () => {
  class FirstBase {}
  class SecondBase {}
  class Initial extends FirstBase {
    value(): string { return "initial"; }
  }
  class Replacement extends SecondBase {
    value(): string { return "replacement"; }
  }

  assert.equal(registerHotReloadableClass("test/incompatible-class", Initial), "registered");
  const instance = new Initial();
  assert.equal(registerHotReloadableClass("test/incompatible-class", Replacement), "incompatible");
  assert.equal(instance.value(), "initial");
});

test("hot reload validates class identities", () => {
  class Example {}
  assert.throws(() => registerHotReloadableClass("", Example), /ID must not be empty/u);
  assert.equal(registerHotReloadableClass("test/unchanged-class", Example), "registered");
  assert.equal(registerHotReloadableClass("test/unchanged-class", Example), "unchanged");
});

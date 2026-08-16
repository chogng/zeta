import assert from "node:assert/strict";
import test from "node:test";
import { analyzeHotReloadModule, unsafeHotReloadChangeReason } from "./hotReloadAnalysis.ts";
import { hotReloadPlugin } from "./hotReloadPlugin.ts";

test("Vite hot reload injects setup and a generic export-handler boundary", () => {
  const plugin = hotReloadPlugin({ desktopRoot: "/workspace/desktop", setupPath: "/workspace/desktop/build/vite/setup-dev.ts" });
  const transformed = transform(plugin, "export class SidebarPart extends PaneCompositePart {}", "/workspace/desktop/src/zeta/sidebarPart.ts?direct");
  const htmlTags = plugin.transformIndexHtml.handler("", {});

  assert.equal(typeof transformed, "string");
  assert.match(transformed, /src\/zeta\/sidebarPart\.ts/u);
  assert.match(transformed, /__zetaViteHotReloadExports/u);
  assert.match(transformed, /\$hotReload_applyNewExports/u);
  assert.match(transformed, /import\.meta\.hot\.accept/u);
  assert.doesNotMatch(transformed, /\$zetaHotReload_registerClass/u);
  assert.deepEqual(htmlTags, [{ tag: "script", attrs: { type: "module", src: "/@fs/workspace/desktop/build/vite/setup-dev.ts" }, injectTo: "head-prepend" }]);
});

test("Vite hot reload supports explicit prototype-patch opt in", () => {
  const plugin = hotReloadPlugin({ desktopRoot: "/workspace/desktop" });
  const transformed = transform(plugin, "// @zeta-hot-reload patch-prototype\nexport class CustomSurface extends BaseSurface {}", "/workspace/desktop/src/zeta/customSurface.ts");
  assert.match(transformed, /CustomSurface/u);
});

test("Vite hot reload leaves non-UI modules and production builds unchanged", () => {
  const plugin = hotReloadPlugin({ desktopRoot: "/workspace/desktop" });
  assert.equal(plugin.apply, "serve");
  assert.equal(transform(plugin, "export class ConfigurationService extends DisposableOwner {}", "/workspace/desktop/src/zeta/configurationService.ts"), undefined);
  assert.equal(transform(plugin, "export class SidebarPart extends PaneCompositePart {}", "/workspace/desktop/src/zeta/sidebarPart.js"), undefined);
});

test("Vite hot reload accepts instance method and accessor changes", () => {
  const before = analyzeHotReloadModule("export class SidebarPart extends BasePart { render() { return 1; } get label() { return 'one'; } }");
  const after = analyzeHotReloadModule("export class SidebarPart extends BasePart { paint() { return 2; } get label() { return 'two'; } }");
  assert.equal(unsafeHotReloadChangeReason(before, after), undefined);
});

test("Vite hot reload rejects initialization and module-boundary changes", () => {
  const base = analyzeHotReloadModule("import { value } from './value.js'; export class SidebarPart extends BasePart { label = value; render() {} }");
  const constructorChanged = analyzeHotReloadModule("import { value } from './value.js'; export class SidebarPart extends BasePart { label = value; constructor() { super(); } render() {} }");
  const fieldChanged = analyzeHotReloadModule("import { value } from './value.js'; export class SidebarPart extends BasePart { label = value + 'changed'; render() {} }");
  const staticChanged = analyzeHotReloadModule("import { value } from './value.js'; export class SidebarPart extends BasePart { static kind = 'sidebar'; label = value; render() {} }");
  const importChanged = analyzeHotReloadModule("import { value } from './other.js'; export class SidebarPart extends BasePart { label = value; render() {} }");

  assert.match(unsafeHotReloadChangeReason(base, constructorChanged), /constructor/u);
  assert.match(unsafeHotReloadChangeReason(base, fieldChanged), /field/u);
  assert.match(unsafeHotReloadChangeReason(base, staticChanged), /static/u);
  assert.match(unsafeHotReloadChangeReason(base, importChanged), /module/u);
});

test("Vite hot reload sends a full reload before an unsafe module executes", async () => {
  const plugin = hotReloadPlugin({ desktopRoot: "/workspace/desktop" });
  const file = "/workspace/desktop/src/zeta/sidebarPart.ts";
  transform(plugin, "export class SidebarPart extends BasePart { render() {} }", file);
  const messages = [];
  const logs = [];
  const result = await plugin.handleHotUpdate({
    file,
    read: async () => "export class SidebarPart extends OtherPart { render() {} }",
    server: { config: { logger: { info: message => logs.push(message) } }, ws: { send: message => messages.push(message) } },
  });

  assert.deepEqual(result, []);
  assert.deepEqual(messages, [{ type: "full-reload", path: "*" }]);
  assert.match(logs[0], /inheritance/u);
});

test("Vite hot reload keeps HMR for a safe method-only update", async () => {
  const plugin = hotReloadPlugin({ desktopRoot: "/workspace/desktop" });
  const file = "/workspace/desktop/src/zeta/sidebarPart.ts";
  transform(plugin, "export class SidebarPart extends BasePart { render() { return 1; } }", file);
  const messages = [];
  const result = await plugin.handleHotUpdate({
    file,
    read: async () => "export class SidebarPart extends BasePart { render() { return 2; } }",
    server: { config: { logger: { info() {} } }, ws: { send: message => messages.push(message) } },
  });
  assert.equal(result, undefined);
  assert.deepEqual(messages, []);
});

function transform(plugin, code, id) {
  return plugin.transform.handler(code, id);
}

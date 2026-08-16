import assert from "node:assert/strict";
import test from "node:test";

import { workbenchHotReloadPlugin } from "./workbench-hot-reload-vite-plugin.mjs";

test("Workbench hot reload instruments conventional persistent UI classes", () => {
  const plugin = workbenchHotReloadPlugin({ root: "/workspace/desktop" });
  const transformed = plugin.transform.handler(
    "export class SidebarPart extends PaneCompositePart {}",
    "/workspace/desktop/src/zeta/workbench/browser/parts/sidebarPart.ts?direct",
  );

  assert.equal(typeof transformed, "string");
  assert.match(transformed, /src\/zeta\/workbench\/browser\/parts\/sidebarPart\.ts#SidebarPart/u);
  assert.match(transformed, /\$zetaHotReload_registerClass/u);
  assert.match(transformed, /import\.meta\.hot\.accept\(\)/u);
  assert.match(transformed, /import\.meta\.hot\.invalidate/u);
});

test("Workbench hot reload supports explicit prototype-patch opt in", () => {
  const plugin = workbenchHotReloadPlugin({ root: "/workspace/desktop" });
  const transformed = plugin.transform.handler(
    "// @zeta-hot-reload patch-prototype\nexport class CustomSurface extends BaseSurface {}",
    "/workspace/desktop/src/zeta/customSurface.ts",
  );

  assert.equal(typeof transformed, "string");
  assert.match(transformed, /src\/zeta\/customSurface\.ts#CustomSurface/u);
});

test("Workbench hot reload leaves non-UI modules and production builds unchanged", () => {
  const plugin = workbenchHotReloadPlugin({ root: "/workspace/desktop" });
  assert.equal(plugin.apply, "serve");
  assert.equal(plugin.transform.handler(
    "export class ConfigurationService extends DisposableOwner {}",
    "/workspace/desktop/src/zeta/configurationService.ts",
  ), undefined);
  assert.equal(plugin.transform.handler(
    "export class SidebarPart extends PaneCompositePart {}",
    "/workspace/desktop/src/zeta/sidebarPart.js",
  ), undefined);
});

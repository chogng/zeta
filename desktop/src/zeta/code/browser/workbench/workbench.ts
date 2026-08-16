import "../../../workbench/workbench.web.main.js";
import type { ProductId } from "../../../product/common/product.js";

declare const __ZETA_PRODUCT__: ProductId;

if (__ZETA_PRODUCT__ === "code") {
  await import("./modes/code.js");
} else {
  await import("./modes/academic.js");
}

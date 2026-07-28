import "../../../workbench/workbench.web.main.js";
import "../../../editor/prosemirror/contrib/proseMirrorEditor.contribution.js";
import {
  AcademicProduct,
} from "../../../product/common/product.js";
import {
  startWebWorkbench,
} from "../../../workbench/browser/web.factory.js";

startWebWorkbench(AcademicProduct);

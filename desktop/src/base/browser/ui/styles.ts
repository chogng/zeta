const styleId = "zeta-base-ui-styles";

/** Installs the small default stylesheet used by base UI components once per document. */
export function installBaseUiStyles(document: Document = window.document): void {
  if (document.getElementById(styleId)) return;
  const style = document.createElement("link");
  style.id = styleId;
  style.rel = "stylesheet";
  style.href = new URL("../../../../../src/base/browser/ui/styles.css", import.meta.url).href;
  document.head.append(style);
}

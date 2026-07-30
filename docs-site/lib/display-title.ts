import type { ZetaDoc } from "@/lib/types";

const acronymNames: Array<[RegExp, string]> = [
  [/^rs\b/i, "Rust"],
  [/^api\b/i, "API"],
  [/^cli\b/i, "CLI"],
  [/^mcp\b/i, "MCP"],
  [/^pdf\b/i, "PDF"],
  [/^tui\b/i, "TUI"],
  [/^ui\b/i, "UI"],
];

export function displayTitle(doc: Pick<ZetaDoc, "slug" | "title">) {
  if (doc.slug.startsWith("crates/")) return doc.title;

  let title = doc.title.replace(/^Zeta\s+/i, "").replace(/^zeta-/i, "");
  for (const [pattern, replacement] of acronymNames) {
    if (pattern.test(title)) {
      title = title.replace(pattern, replacement);
      break;
    }
  }

  return title.replace(/^[a-z]/, (character) => character.toUpperCase());
}

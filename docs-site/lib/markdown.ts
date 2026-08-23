import MarkdownIt from "markdown-it";
import markdownItAnchor from "markdown-it-anchor";
import { docs } from "@zeta-generated-docs";
import type { ZetaDoc } from "@/lib/types";

const sourceToSlug = new Map(docs.map((doc) => [doc.sourcePath, doc.slug]));

function slugify(value: string) {
  return value
    .replace(/`([^`]+)`/g, "$1")
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .trim()
    .toLowerCase()
    .replace(/\s+/g, "-");
}

function resolveSourcePath(sourcePath: string, target: string) {
  const sourceUrl = new URL(sourcePath, "https://zeta.local/");
  return new URL(target, sourceUrl).pathname.replace(/^\//, "");
}

export function renderMarkdown(doc: ZetaDoc) {
  const md = new MarkdownIt({
    html: false,
    linkify: true,
    typographer: true,
  }).use(markdownItAnchor, { slugify });

  const defaultLinkOpen = md.renderer.rules.link_open;
  md.renderer.rules.link_open = (tokens, index, options, environment, self) => {
    const href = tokens[index].attrGet("href");
    if (href && !href.startsWith("#") && !/^[a-z][a-z\d+.-]*:/i.test(href)) {
      const [target, hash] = href.split("#", 2);
      const resolvedSource = resolveSourcePath(doc.sourcePath, target);
      const targetSlug = sourceToSlug.get(resolvedSource);
      if (targetSlug) {
        tokens[index].attrSet("href", `/docs/${targetSlug}${hash ? `#${hash}` : ""}`);
      } else {
        tokens[index].attrSet("href", `https://github.com/chogng/zeta/blob/main/${resolvedSource}${hash ? `#${hash}` : ""}`);
        tokens[index].attrSet("target", "_blank");
        tokens[index].attrSet("rel", "noreferrer");
      }
    } else if (href && /^https?:\/\//.test(href)) {
      tokens[index].attrSet("target", "_blank");
      tokens[index].attrSet("rel", "noreferrer");
    }
    return defaultLinkOpen ? defaultLinkOpen(tokens, index, options, environment, self) : self.renderToken(tokens, index, options);
  };

  return md.render(doc.markdown);
}

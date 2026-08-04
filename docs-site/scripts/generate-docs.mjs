import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const siteRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = resolve(siteRoot, "..");
const docsDirectory = join(repositoryRoot, "docs");
const outputPath = join(siteRoot, "app", "generated-docs.ts");

const groups = [
  {
    label: "开始了解",
    files: [
      "README",
      "architecture",
      "product-editions",
      "product-lines",
      "zeta-rs-architecture",
      "zeta-desktop-architecture",
      "zeta-cli-architecture",
      "zeta-agent-runtime-architecture",
    ],
  },
  {
    label: "界面与体验",
    files: [
      "ui-styling-ownership",
      "workbench-pane-composite-design",
      "design-tokens",
      "theme-authoring-template",
      "menu-system",
      "icons",
      "search",
      "syntax-analysis",
      "lsp",
      "editor-architecture",
      "editor-core",
      "pdf",
      "typst",
      "chat-session-inspector",
      "tui",
    ],
  },
  {
    label: "zeterm 产品",
    files: [
      "zeterm/README",
      "zeterm/native-agent-console",
      "zeterm/native-terminal-ui",
      "zeterm/native-text-input",
      "zeterm/rendering-architecture",
      "zeterm/ui-component-migration-plan",
      "zeterm/native-deprecation-plan",
      "zeterm/zeterm-app-migration-plan",
      "zeterm/zeterm-release-graph",
    ],
  },
  {
    label: "Agent 与运行时",
    files: [
      "core",
      "agent-harness-design",
      "agent-tools-spec",
      "agent-harness-implementation-plan",
      "core-context",
      "core-multi-agent",
      "exec",
      "tools",
      "skills",
      "plugins",
      "slash-commands",
      "mcp",
      "mcp-server",
    ],
  },
  {
    label: "安全与权限",
    files: [
      "permissions",
      "auto-review",
      "sandboxing",
      "workspace-security",
      "secrets",
      "windows-sandbox-acceptance-runbook",
    ],
  },
  {
    label: "API 与协议",
    files: [
      "zeta-api",
      "zeta-api-interface-requirements",
      "zeta-api-interface-template",
      "zeta-app-server-api",
      "app-server-client",
      "codex-app-server",
      "protocol",
      "zeta-client",
    ],
  },
  {
    label: "模型与配置",
    files: [
      "models-manager",
      "model-provider",
      "model-provider-config",
      "config",
      "login",
    ],
  },
  {
    label: "平台与交付",
    files: [
      "git",
      "documentation-guidelines",
    ],
  },
  {
    label: "计划与迁移",
    files: [
      "zeta-code-architecture-codex-style-v2",
    ],
  },
];

function walkReadmes(directory, ignoredDirectories = new Set()) {
  const results = [];
  for (const entry of readdirSync(directory)) {
    if (entry === "target" || entry === "node_modules" || entry.startsWith(".")) continue;
    const path = join(directory, entry);
    const stats = statSync(path);
    if (stats.isDirectory()) {
      if (ignoredDirectories.has(entry)) continue;
      results.push(...walkReadmes(path, ignoredDirectories));
    } else if (entry === "README.md") {
      results.push(path);
    }
  }
  return results;
}

function cleanInlineMarkdown(value) {
  return value
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/[*_~]/g, "")
    .trim();
}

function slugify(value) {
  return cleanInlineMarkdown(value)
    .toLowerCase()
    .replace(/[^\p{Letter}\p{Number}\s-]/gu, "")
    .trim()
    .replace(/\s+/g, "-");
}

function extractDescription(lines) {
  const paragraph = [];
  let fenced = false;
  for (const line of lines) {
    if (line.startsWith("```")) {
      fenced = !fenced;
      continue;
    }
    if (fenced || !line.trim()) {
      if (paragraph.length) break;
      continue;
    }
    if (/^(#|>|[-*+] |\d+\. |\|)/.test(line.trim())) continue;
    paragraph.push(cleanInlineMarkdown(line));
    if (paragraph.join(" ").length > 180) break;
  }
  const value = paragraph.join(" ").replace(/\s+/g, " ").trim();
  return value.length > 190 ? `${value.slice(0, 187)}…` : value;
}

function parseDocument(path, slug, group) {
  const sourcePath = relative(repositoryRoot, path).replaceAll("\\", "/");
  const raw = readFileSync(path, "utf8").replace(/\r\n/g, "\n");
  const lines = raw.split("\n");
  const titleLine = lines.findIndex((line) => /^#\s+/.test(line));
  const title = titleLine >= 0 ? cleanInlineMarkdown(lines[titleLine].replace(/^#\s+/, "")) : basename(path, ".md");
  const bodyLines = titleLine >= 0 ? lines.filter((_, index) => index !== titleLine) : lines;
  const usedIds = new Map();
  const headings = [];

  for (const line of bodyLines) {
    const match = /^(#{2,3})\s+(.+?)\s*$/.exec(line);
    if (!match) continue;
    const headingTitle = cleanInlineMarkdown(match[2]);
    const baseId = slugify(headingTitle) || "section";
    const count = usedIds.get(baseId) ?? 0;
    usedIds.set(baseId, count + 1);
    headings.push({ depth: match[1].length, id: count ? `${baseId}-${count}` : baseId, title: headingTitle });
  }

  const markdown = bodyLines.join("\n").trim();
  const searchText = cleanInlineMarkdown(markdown)
    .replace(/[#>|()[\]{}]/g, " ")
    .replace(/\s+/g, " ")
    .slice(0, 12000);

  return {
    slug,
    sourcePath,
    title,
    description: extractDescription(bodyLines) || "Zeta 工程文档",
    group,
    markdown,
    headings,
    searchText,
  };
}

const systemDocRoots = [
  { directory: docsDirectory, prefix: "" },
  { directory: join(repositoryRoot, "zeterm", "docs"), prefix: "zeterm/" },
];
const systemDocEntries = systemDocRoots.flatMap(({ directory, prefix }) =>
  readdirSync(directory)
    .filter((name) => name.endsWith(".md"))
    .map((name) => ({ path: join(directory, name), slug: `${prefix}${basename(name, ".md")}` })),
);
const groupBySlug = new Map(groups.flatMap((group) => group.files.map((slug) => [slug, group.label])));
const knownSlugs = new Set(groupBySlug.keys());
const ungrouped = systemDocEntries.map(({ slug }) => slug).filter((slug) => !knownSlugs.has(slug)).sort();
if (ungrouped.length) groups.push({ label: "其他系统文档", files: ungrouped });

const systemDocs = systemDocEntries.map(({ path, slug }) =>
  parseDocument(path, slug, groupBySlug.get(slug) ?? "其他系统文档"),
);

const crateRoots = [
  { directory: join(repositoryRoot, "zeta-rs"), prefix: "", ignoredDirectories: [] },
  { directory: join(repositoryRoot, "zeterm"), prefix: "zeterm/", ignoredDirectories: ["docs"] },
];
const crateDocs = crateRoots.flatMap(({ directory, prefix, ignoredDirectories }) => walkReadmes(directory, new Set(ignoredDirectories))
  .map((path) => {
    const cratePath = relative(directory, dirname(path)).replaceAll("\\", "/");
    return parseDocument(path, `crates/${prefix}${cratePath}`, "Crate 实现参考");
  }))
  .sort((left, right) => left.title.localeCompare(right.title));

const allDocs = [...systemDocs, ...crateDocs];
const documentBySlug = new Map(allDocs.map((doc) => [doc.slug, doc]));
const orderedGroups = groups.map((group) => ({
  label: group.label,
  slugs: group.files.filter((slug) => documentBySlug.has(slug)),
}));
orderedGroups.push({ label: "Crate 实现参考", slugs: crateDocs.map((doc) => doc.slug) });
const orderedDocs = orderedGroups.flatMap((group) => group.slugs.map((slug) => documentBySlug.get(slug)));

const output = `// This file is generated by scripts/generate-docs.mjs. Do not edit by hand.
import type { DocGroup, ZetaDoc } from "@/lib/types";

export const docs: ZetaDoc[] = ${JSON.stringify(orderedDocs, null, 2)};
export const docGroups: DocGroup[] = ${JSON.stringify(orderedGroups, null, 2)};
`;

writeFileSync(outputPath, output);
console.log(`Generated ${orderedDocs.length} Zeta documentation pages.`);

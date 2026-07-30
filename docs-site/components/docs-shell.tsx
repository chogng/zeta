"use client";

import Link from "next/link";
import { useEffect, useMemo, useRef, useState } from "react";
import { displayTitle } from "@/lib/display-title";
import type { DocGroup, SearchItem, ZetaDoc } from "@/lib/types";

type DocsShellProps = {
  currentDoc: ZetaDoc;
  currentIndex: number;
  groups: DocGroup[];
  html: string;
  nextDoc: ZetaDoc | null;
  previousDoc: ZetaDoc | null;
  searchItems: SearchItem[];
};

export function DocsShell({ currentDoc, currentIndex, groups, html, nextDoc, previousDoc, searchItems }: DocsShellProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [theme, setTheme] = useState<"light" | "dark">("light");
  const prose = useRef<HTMLDivElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const titles = useMemo(() => new Map(searchItems.map((item) => [item.slug, displayTitle(item)])), [searchItems]);
  const activeGroup = groups.find((group) => group.slugs.includes(currentDoc.slug)) ?? { label: "工程文档", slugs: [] };
  const sourceUrl = `https://github.com/chogng/zeta/edit/main/${currentDoc.sourcePath}`;

  useEffect(() => {
    const saved = window.localStorage.getItem("zeta-docs-theme");
    const initialTheme = saved === "dark" || (!saved && window.matchMedia("(prefers-color-scheme: dark)").matches) ? "dark" : "light";
    document.documentElement.dataset.theme = initialTheme;
    const frame = window.requestAnimationFrame(() => setTheme(initialTheme));
    return () => window.cancelAnimationFrame(frame);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchOpen(true);
      }
      if (event.key === "Escape") {
        setSearchOpen(false);
        setMenuOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (searchOpen) window.setTimeout(() => searchInput.current?.focus(), 20);
  }, [searchOpen]);

  useEffect(() => {
    let cancelled = false;

    const renderDiagrams = async () => {
      const root = prose.current;
      if (!root) return;

      root.querySelectorAll<HTMLElement>("pre > code.language-mermaid").forEach((code) => {
        const container = document.createElement("div");
        container.className = "flow-diagram";
        container.dataset.mermaidSource = code.textContent ?? "";
        container.setAttribute("role", "img");
        container.setAttribute("aria-label", "文档流程图");
        code.parentElement?.replaceWith(container);
      });

      const diagrams = Array.from(root.querySelectorAll<HTMLElement>(".flow-diagram[data-mermaid-source]"));
      if (!diagrams.length) return;

      const { default: mermaid } = await import("mermaid");
      if (cancelled) return;

      const styles = window.getComputedStyle(document.documentElement);
      const token = (name: string) => styles.getPropertyValue(name).trim();
      mermaid.initialize({
        startOnLoad: false,
        securityLevel: "strict",
        theme: "base",
        themeVariables: {
          background: token("--background"),
          primaryColor: token("--accent-soft"),
          primaryTextColor: token("--foreground"),
          primaryBorderColor: token("--accent"),
          secondaryColor: token("--surface"),
          secondaryTextColor: token("--foreground"),
          secondaryBorderColor: token("--border-strong"),
          tertiaryColor: token("--surface-strong"),
          tertiaryTextColor: token("--foreground"),
          tertiaryBorderColor: token("--border-strong"),
          lineColor: token("--muted"),
          edgeLabelBackground: token("--background"),
          clusterBkg: token("--surface"),
          clusterBorder: token("--border-strong"),
          fontFamily: 'Inter, ui-sans-serif, "PingFang SC", "Microsoft YaHei", sans-serif',
          fontSize: "14px",
        },
      });

      for (const [index, container] of diagrams.entries()) {
        const source = container.dataset.mermaidSource;
        if (!source) continue;
        try {
          const diagramId = `diagram-${currentDoc.slug.replace(/[^a-z0-9-]/gi, "-")}-${theme}-${index}`;
          const { svg, bindFunctions } = await mermaid.render(diagramId, source);
          if (cancelled) return;
          container.innerHTML = svg;
          bindFunctions?.(container);
        } catch {
          container.classList.add("flow-diagram-error");
          container.textContent = "流程图暂时无法渲染。";
        }
      }
    };

    void renderDiagrams();
    return () => {
      cancelled = true;
    };
  }, [currentDoc.slug, html, theme]);

  const results = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) return searchItems.slice(0, 8);
    return searchItems
      .map((item) => {
        const title = item.title.toLowerCase();
        const haystack = `${item.title} ${item.description} ${item.group} ${item.searchText}`.toLowerCase();
        const score = title === normalized ? 100 : title.includes(normalized) ? 50 : haystack.includes(normalized) ? 10 : 0;
        return { item, score };
      })
      .filter((result) => result.score > 0)
      .sort((left, right) => right.score - left.score)
      .slice(0, 12)
      .map((result) => result.item);
  }, [query, searchItems]);

  const toggleTheme = () => {
    const nextTheme = theme === "light" ? "dark" : "light";
    setTheme(nextTheme);
    document.documentElement.dataset.theme = nextTheme;
    window.localStorage.setItem("zeta-docs-theme", nextTheme);
  };

  const sidebar = (
    <nav className="sidebar-nav" aria-label="文档导航">
      <div className="sidebar-context">
        <span className="sidebar-eyebrow">当前分类</span>
        <div className="sidebar-section-heading"><span>{activeGroup.label}</span><span>{activeGroup.slugs.length}</span></div>
      </div>
      <div className="nav-items">
        {activeGroup.slugs.map((slug) => (
          <Link className={slug === currentDoc.slug ? "nav-link active" : "nav-link"} href={`/docs/${slug}`} key={slug} onClick={() => setMenuOpen(false)}>
            {titles.get(slug)}
          </Link>
        ))}
      </div>
    </nav>
  );

  return (
    <div className="site-shell">
      <header className="topbar">
        <div className="topbar-main">
          <button className="icon-button mobile-only" type="button" aria-label="打开导航" onClick={() => setMenuOpen(true)}>☰</button>
          <Link className="brand" href="/docs/architecture" aria-label="Zeta 文档首页">
            <span className="brand-mark">Z</span>
            <span>Zeta</span>
            <span className="brand-divider" />
            <span className="brand-subtitle">Docs</span>
          </Link>
          <button className="search-trigger" type="button" onClick={() => setSearchOpen(true)}>
            <span>搜索文档</span><kbd>⌘ K</kbd>
          </button>
          <div className="top-actions">
            <a className="github-link" href="https://github.com/chogng/zeta" target="_blank" rel="noreferrer">GitHub ↗</a>
            <button className="icon-button" type="button" aria-label="切换明暗主题" onClick={toggleTheme}>{theme === "light" ? "◐" : "☀"}</button>
          </div>
        </div>
        <div className="topbar-navigation">
          <nav className="global-nav" aria-label="一级文档导航">
            {groups.map((group) => (
              <Link className={group === activeGroup ? "global-nav-link active" : "global-nav-link"} href={`/docs/${group.slugs[0]}`} key={group.label}>
                {group.label}
              </Link>
            ))}
          </nav>
        </div>
      </header>

      <aside className="sidebar desktop-sidebar">{sidebar}</aside>
      {menuOpen && (
        <div className="mobile-drawer-backdrop" role="presentation" onClick={() => setMenuOpen(false)}>
          <aside className="mobile-drawer" onClick={(event) => event.stopPropagation()}>
            <div className="drawer-header"><strong>{activeGroup.label}</strong><button className="icon-button" type="button" aria-label="关闭导航" onClick={() => setMenuOpen(false)}>×</button></div>
            {sidebar}
          </aside>
        </div>
      )}

      <main className="content-layout">
        <article className="doc">
          <div className="doc-meta">
            <div className="breadcrumbs"><span>{currentDoc.group}</span><span>/</span><span>第 {currentIndex + 1} 篇</span></div>
            <a className="source-reference" href={sourceUrl} target="_blank" rel="noreferrer" aria-label={`在 GitHub 编辑 ${currentDoc.sourcePath}`} title="在 GitHub 编辑源文档">
              <span className="source-reference-label">内容来源</span>
              <code className="source-path">{currentDoc.sourcePath}</code>
              <span className="source-link-icon" aria-hidden="true">↗</span>
            </a>
          </div>
          <h1>{displayTitle(currentDoc)}</h1>
          <p className="doc-description">{currentDoc.description}</p>
          <div ref={prose} className="prose" dangerouslySetInnerHTML={{ __html: html }} />
          <nav className="page-navigation" aria-label="上一篇和下一篇">
            {previousDoc ? <Link href={`/docs/${previousDoc.slug}`}><small>上一篇</small><span>← {displayTitle(previousDoc)}</span></Link> : <span />}
            {nextDoc ? <Link className="next" href={`/docs/${nextDoc.slug}`}><small>下一篇</small><span>{displayTitle(nextDoc)} →</span></Link> : <span />}
          </nav>
          <footer className="doc-footer">Zeta 文档由仓库中的 Markdown 自动生成。</footer>
        </article>

        <aside className="toc">
          <div className="toc-title">本页内容</div>
          {currentDoc.headings.length ? currentDoc.headings.map((heading) => (
            <a className={heading.depth === 3 ? "toc-link nested" : "toc-link"} href={`#${heading.id}`} key={`${heading.id}-${heading.title}`}>{heading.title}</a>
          )) : <span className="toc-empty">本页没有分节</span>}
        </aside>
      </main>

      {searchOpen && (
        <div className="search-backdrop" role="presentation" onClick={() => setSearchOpen(false)}>
          <div className="search-dialog" role="dialog" aria-modal="true" aria-label="搜索文档" onClick={(event) => event.stopPropagation()}>
            <div className="search-input-row">
              <span>⌕</span>
              <input ref={searchInput} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索架构、API、组件或术语…" aria-label="搜索内容" />
              <kbd>ESC</kbd>
            </div>
            <div className="search-results">
              {results.length ? results.map((item) => (
                <Link href={`/docs/${item.slug}`} key={item.slug} onClick={() => setSearchOpen(false)}>
                  <span className="result-group">{item.group}</span>
                  <strong>{displayTitle(item)}</strong>
                  <p>{item.description}</p>
                </Link>
              )) : <div className="empty-results">没有找到相关文档。试试更短的关键词。</div>}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

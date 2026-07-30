# Zeta Docs

Zeta Docs turns the repository's existing Markdown into a searchable documentation website.
The site presentation lives here; canonical content remains in:

- `../docs/*.md` for cross-crate architecture and product semantics;
- `../zeta-rs/**/README.md` for crate implementation contracts.

`scripts/generate-docs.mjs` reads those sources before local development and production builds,
then creates `app/generated-docs.ts`. Do not edit that generated module by hand.

## Local preview

```bash
npm install
npm run dev
```

The development server prints the local URL. Changes to Markdown require restarting the server or
running `npm run generate:docs`.

## Validation

```bash
npm test
```

This regenerates the documentation data, builds the production worker, and verifies that the
documentation index is rendered with the expected metadata and navigation.

## Ownership

- `scripts/generate-docs.mjs` owns source discovery, navigation grouping, titles, descriptions,
  headings, and search text.
- `lib/markdown.ts` owns Markdown rendering and repository-link rewriting.
- `components/docs-shell.tsx` owns navigation, search, theme selection, and responsive behavior.
- `app/globals.css` owns the documentation site's visual system.
- `../docs/documentation-guidelines.md` remains canonical for documentation content and structure.

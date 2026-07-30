export type DocHeading = {
  depth: number;
  id: string;
  title: string;
};

export type ZetaDoc = {
  slug: string;
  sourcePath: string;
  title: string;
  description: string;
  group: string;
  markdown: string;
  headings: DocHeading[];
  searchText: string;
};

export type DocGroup = {
  label: string;
  slugs: string[];
};

export type SearchItem = Pick<ZetaDoc, "slug" | "title" | "description" | "group" | "searchText">;

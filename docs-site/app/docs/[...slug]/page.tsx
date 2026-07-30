import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { docGroups, docs } from "@/app/generated-docs";
import { DocsShell } from "@/components/docs-shell";
import { displayTitle } from "@/lib/display-title";
import { renderMarkdown } from "@/lib/markdown";

type PageProps = {
  params: Promise<{ slug: string[] }>;
};

export function generateStaticParams() {
  return docs.map((doc) => ({ slug: doc.slug.split("/") }));
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const doc = docs.find((candidate) => candidate.slug === slug.join("/"));
  return doc ? { title: displayTitle(doc), description: doc.description } : {};
}

export default async function DocPage({ params }: PageProps) {
  const { slug } = await params;
  const currentSlug = slug.join("/");
  const currentIndex = docs.findIndex((doc) => doc.slug === currentSlug);
  if (currentIndex < 0) notFound();

  const doc = docs[currentIndex];
  const searchItems = docs.map(({ slug, title, description, group, searchText }) => ({ slug, title, description, group, searchText }));

  return (
    <DocsShell
      currentDoc={doc}
      currentIndex={currentIndex}
      groups={docGroups}
      html={renderMarkdown(doc)}
      nextDoc={docs[currentIndex + 1] ?? null}
      previousDoc={docs[currentIndex - 1] ?? null}
      searchItems={searchItems}
    />
  );
}

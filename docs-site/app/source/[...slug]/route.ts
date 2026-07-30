import { docs } from "@/app/generated-docs";

type RouteContext = {
  params: Promise<{ slug: string[] }>;
};

export async function GET(_request: Request, { params }: RouteContext) {
  const { slug } = await params;
  const doc = docs.find((candidate) => candidate.slug === slug.join("/"));
  if (!doc) return new Response("Document source not found.\n", { status: 404 });

  const filename = doc.sourcePath.split("/").at(-1) ?? "document.md";
  const source = `# ${doc.title}\n\n${doc.markdown}\n`;

  return new Response(source, {
    headers: {
      "cache-control": "public, max-age=300",
      "content-disposition": `inline; filename*=UTF-8''${encodeURIComponent(filename)}`,
      "content-type": "text/markdown; charset=utf-8",
    },
  });
}

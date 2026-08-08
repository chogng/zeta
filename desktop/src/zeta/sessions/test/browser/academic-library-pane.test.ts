import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { AcademicLibraryPane } from "../../browser/academic/academicLibraryPane.js";

test("Academic library imports PDF, BibTeX, and RIS references into its own session surface", async () => {
  const environment = new JSDOM("<!doctype html><body></body>");
  try {
    using library = new AcademicLibraryPane(environment.window.document);
    const selections: string[] = [];
    using selected = library.onDidSelectItem((item) => {
      if (item) selections.push(`${item.kind}:${item.title}`);
    });
    const pdf = new environment.window.File(["%PDF"], "paper-one.pdf", { type: "application/pdf", lastModified: 1 });
    const bib = new environment.window.File(["@article{example}"], "important-result.bib", { type: "text/plain", lastModified: 2 });
    const ris = new environment.window.File(["TY  - JOUR"], "review.ris", { type: "text/plain", lastModified: 3 });

    await library.importFiles([pdf as unknown as File, bib as unknown as File, ris as unknown as File]);

    assert.equal(library.element.querySelectorAll(".zeta-academic-library-item").length, 3);
    assert.equal(library.selectedItem?.kind, "pdf");
    assert.equal(library.selectedItem?.title, "paper one");
    assert.deepEqual(selections, ["pdf:paper one"]);
  } finally {
    environment.window.close();
  }
});

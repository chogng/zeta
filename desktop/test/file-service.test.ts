import assert from "node:assert/strict";
import test from "node:test";
import { URI } from "../src/zeta/base/common/uri.js";
import {
  BrowserFileService,
  workspaceRelativePath,
} from "../src/zeta/platform/files/browser/fileService.js";
import {
  FileKind,
} from "../src/zeta/platform/files/common/files.js";
import type {
  IWorkspaceContextService,
} from "../src/zeta/platform/workspace/common/workspace.js";

test("workspaceRelativePath confines resources to the folder", () => {
  const root = URI.file("C:\\project");
  assert.equal(workspaceRelativePath(root, URI.file("C:\\project")), "");
  assert.equal(
    workspaceRelativePath(root, URI.file("C:\\project\\src\\main.ts")),
    "src/main.ts",
  );
  assert.throws(
    () => workspaceRelativePath(root, URI.file("C:\\project-other\\file.ts")),
    /outside/,
  );
});

test("BrowserFileService maps wire entries back to resource URIs", async () => {
  const root = URI.file("C:\\project");
  const workspaceContextService: IWorkspaceContextService = {
    getWorkspace: () => ({
      id: "workspace",
      folders: [{ uri: root, name: "project", index: 0 }],
    }),
    getWorkbenchState: () => 2,
  };
  const service = new BrowserFileService({
    workspaceContextService,
    api: {
      getMetadata: async ({ path }) => {
        assert.equal(path, "");
        return {
          fileType: "directory",
          sizeBytes: 0,
          readonly: false,
          modifiedAtMillis: null,
        };
      },
      readDirectory: async ({ path }) => {
        assert.equal(path, "src");
        return {
          entries: [{ name: "main.ts", fileType: "file" }],
        };
      },
      readFile: async ({ path }) => {
        assert.equal(path, "src/main.ts");
        return { content: "export {};" };
      },
    },
  });

  assert.equal((await service.stat(root)).kind, FileKind.Directory);
  assert.deepEqual(
    await service.readDirectory(URI.file("C:\\project\\src")),
    [{
      resource: URI.file("C:\\project\\src\\main.ts"),
      name: "main.ts",
      kind: FileKind.File,
    }],
  );
  assert.equal(
    await service.readFile(URI.file("C:\\project\\src\\main.ts")),
    "export {};",
  );
});

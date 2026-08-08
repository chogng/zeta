import assert from "node:assert/strict";
import test from "node:test";
import { Emitter } from "../../../../base/common/event.js";
import { URI } from "../../../../base/common/uri.js";
import { BrowserFileService, workspaceRelativePath } from "../../../../platform/files/browser/fileService.js";
import { FileKind, FileRevisionConflictError } from "../../../../platform/files/common/files.js";
import type { FsChanged } from "../../../../../../generated/app-server/types.js";
import type { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { WorkspaceContextService } from "../../../../workbench/services/workspaces/browser/workspaceContextService.js";

test("workspaceRelativePath confines resources to the folder", () => {
  const root = URI.file("C:\\project");
  assert.equal(workspaceRelativePath(root, URI.file("C:\\project")), ".");
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
  using workspaceContextService: IWorkspaceContextService =
    new WorkspaceContextService({ id: "workspace", uri: root });
  const service = new BrowserFileService({
    workspaceContextService,
    api: {
      getMetadata: async ({ path }) => {
        assert.equal(path, ".");
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
        return { content: "export {};", revision: "revision-read" };
      },
      writeFile: async ({ path, content, expectedRevision }) => {
        assert.equal(path, "src/main.ts");
        assert.equal(content, "export const saved = true;");
        assert.equal(expectedRevision, "revision-read");
        return {
          metadata: {
            fileType: "file",
            sizeBytes: content.length,
            readonly: false,
            modifiedAtMillis: 123,
          },
          revision: "revision-write",
        };
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
  assert.deepEqual(
    await service.readFile(URI.file("C:\\project\\src\\main.ts")),
    { resource: URI.file("C:\\project\\src\\main.ts"), content: "export {};", revision: "revision-read" },
  );
  assert.deepEqual(
    await service.writeFile({
      resource: URI.file("C:\\project\\src\\main.ts"),
      content: "export const saved = true;",
      expectedRevision: "revision-read",
    }),
    {
      stat: {
        resource: URI.file("C:\\project\\src\\main.ts"),
        kind: FileKind.File,
        sizeBytes: 26,
        readonly: false,
        modifiedAtMillis: 123,
      },
      revision: "revision-write",
    },
  );
});

test("BrowserFileService maps App Server revision conflicts to the file contract", async () => {
  const resource = URI.file("C:\\project\\src\\main.ts");
  using workspaceContextService: IWorkspaceContextService = new WorkspaceContextService({ id: "workspace", uri: URI.file("C:\\project") });
  const service = new BrowserFileService({
    workspaceContextService,
    api: {
      getMetadata: async () => { throw new Error("unavailable"); },
      readDirectory: async () => { throw new Error("unavailable"); },
      readFile: async () => { throw new Error("unavailable"); },
      writeFile: async () => { throw new Error("FileSystemRevisionConflict"); },
    },
  });

  await assert.rejects(service.writeFile({ resource, content: "local", expectedRevision: "stale" }), FileRevisionConflictError);
});

test("BrowserFileService maps App Server invalidations to workspace resources", () => {
  const root = URI.file("C:\\project");
  using workspaceContextService: IWorkspaceContextService = new WorkspaceContextService({ id: "workspace", uri: root });
  using changes = new Emitter<FsChanged>();
  using service = new BrowserFileService({
    workspaceContextService,
    api: unavailableFileApi(),
    onDidChange: changes.event,
  });
  const observed: (readonly URI[] | undefined)[] = [];
  using listener = service.onDidChangeFiles(event => observed.push(event.resources));

  changes.fire({ type: "pathsChanged", paths: ["src/main.ts", "src/main.ts", "README.md"] });
  changes.fire({ type: "rescanRequired" });

  assert.deepEqual(observed, [[URI.file("C:\\project\\src\\main.ts"), URI.file("C:\\project\\README.md")], undefined]);
});

function unavailableFileApi() {
  return {
    getMetadata: async () => { throw new Error("unavailable"); },
    readDirectory: async () => { throw new Error("unavailable"); },
    readFile: async () => { throw new Error("unavailable"); },
    writeFile: async () => { throw new Error("unavailable"); },
  };
}

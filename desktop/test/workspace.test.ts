import assert from "node:assert/strict";
import { resolve } from "node:path";
import test from "node:test";
import { URI } from "../src/zeta/base/common/uri.js";
import {
  isSingleFolderWorkspaceIdentifier,
  isWorkspaceIdentifier,
  parseWorkspaceIdentifier,
  serializeWorkspaceIdentifier,
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
  workbenchStateFromWorkspaceIdentifier,
  WorkbenchState,
} from "../src/zeta/platform/workspace/common/workspace.js";
import {
  WorkspaceOpenTargetKind,
} from "../src/zeta/platform/workspaces/common/workspaces.js";
import {
  parseWorkspaceLaunchArguments,
  WorkspacesMainService,
  workspaceContextIpcRoutes,
} from "../src/zeta/platform/workspaces/electron-main/workspacesMainService.js";
import {
  getSingleFolderWorkspaceIdentifier,
  getWorkspaceIdentifier,
  type IWorkspacePathService,
  WorkspacePathKind,
} from "../src/zeta/platform/workspaces/node/workspaces.js";
import {
  WorkspaceContextService,
} from "../src/zeta/workbench/services/workspaces/browser/workspaceContextService.js";

test("workspace launch arguments distinguish automatic and named targets", () => {
  assert.equal(parseWorkspaceLaunchArguments([]), undefined);
  assert.deepEqual(parseWorkspaceLaunchArguments(["project"]), {
    kind: WorkspaceOpenTargetKind.Automatic,
    path: "project",
  });
  assert.deepEqual(parseWorkspaceLaunchArguments(["--folder", "project"]), {
    kind: WorkspaceOpenTargetKind.Folder,
    path: "project",
  });
  assert.deepEqual(
    parseWorkspaceLaunchArguments(["--workspace=team.zeta-workspace"]),
    {
      kind: WorkspaceOpenTargetKind.Workspace,
      path: "team.zeta-workspace",
    },
  );
  assert.deepEqual(parseWorkspaceLaunchArguments(["--", "-project"]), {
    kind: WorkspaceOpenTargetKind.Automatic,
    path: "-project",
  });

  assert.throws(
    () => parseWorkspaceLaunchArguments(["one", "two"]),
    /only one project/,
  );
  assert.throws(
    () => parseWorkspaceLaunchArguments(["--folder"]),
    /requires a path/,
  );
});

test("workspaces service resolves a canonical single-folder identity", async () => {
  const cwd = resolve("launch-root");
  const canonicalPath = resolve("canonical", "project");
  let requestedPath: string | undefined;
  const pathService: IWorkspacePathService = {
    async resolvePath(path) {
      requestedPath = path;
      return {
        kind: WorkspacePathKind.Directory,
        path: canonicalPath,
      };
    },
  };

  const workspace = await new WorkspacesMainService(pathService)
    .resolveStartupWorkspace({
      arguments: ["project"],
      cwd,
    });

  assert.equal(requestedPath, resolve(cwd, "project"));
  assert.ok(isSingleFolderWorkspaceIdentifier(workspace));
  assert.equal(workspace.id.length, 64);
  assert.equal(workspace.uri.toString(), URI.file(canonicalPath).toString());
  assert.equal(
    workbenchStateFromWorkspaceIdentifier(workspace),
    WorkbenchState.FOLDER,
  );

  const context = new WorkspaceContextService(workspace);
  assert.equal(context.getWorkbenchState(), WorkbenchState.FOLDER);
  assert.equal(context.getWorkspace().folders[0]?.name, "project");
});

test("workspaces service recognizes explicit workspace files", async () => {
  const canonicalPath = resolve("canonical", "team.zeta-workspace");
  const pathService: IWorkspacePathService = {
    async resolvePath() {
      return {
        kind: WorkspacePathKind.File,
        path: canonicalPath,
      };
    },
  };

  const workspace = await new WorkspacesMainService(pathService)
    .resolveStartupWorkspace({
      arguments: ["--workspace", "team.zeta-workspace"],
      cwd: resolve("launch-root"),
    });

  assert.ok(isWorkspaceIdentifier(workspace));
  assert.equal(
    workspace.configPath.toString(),
    URI.file(canonicalPath).toString(),
  );
  const context = new WorkspaceContextService(workspace);
  assert.equal(context.getWorkbenchState(), WorkbenchState.WORKSPACE);
  assert.equal(context.getWorkspace().name, "team");
});

test("non-empty workspace identifiers are stable for canonical URIs", () => {
  const folderUri = URI.file(resolve("canonical", "project"));
  const configPath = URI.file(resolve("canonical", "team.zeta-workspace"));

  assert.equal(
    getSingleFolderWorkspaceIdentifier(folderUri).id,
    getSingleFolderWorkspaceIdentifier(folderUri).id,
  );
  assert.equal(
    getWorkspaceIdentifier(configPath).id,
    getWorkspaceIdentifier(configPath).id,
  );
});

test("loose files and launches without a target remain empty", async () => {
  const filePath = resolve("canonical", "notes.txt");
  const pathService: IWorkspacePathService = {
    async resolvePath() {
      return {
        kind: WorkspacePathKind.File,
        path: filePath,
      };
    },
  };
  const service = new WorkspacesMainService(pathService);

  assert.deepEqual(
    await service.resolveStartupWorkspace({
      arguments: [],
      cwd: resolve("launch-root"),
    }),
    UNKNOWN_EMPTY_WINDOW_WORKSPACE,
  );
  const looseFileWorkspace = await service.resolveStartupWorkspace({
    arguments: ["notes.txt"],
    cwd: resolve("launch-root"),
  });
  assert.deepEqual(looseFileWorkspace, UNKNOWN_EMPTY_WINDOW_WORKSPACE);
  assert.equal(
    workbenchStateFromWorkspaceIdentifier(looseFileWorkspace),
    WorkbenchState.EMPTY,
  );
  assert.equal(
    new WorkspaceContextService(looseFileWorkspace).getWorkbenchState(),
    WorkbenchState.EMPTY,
  );
  await assert.rejects(
    service.resolveStartupWorkspace({
      arguments: ["--folder", "notes.txt"],
      cwd: resolve("launch-root"),
    }),
    /not a directory/,
  );
});

test("workspace identifier IPC validation revives URIs", () => {
  const serializedFolder = {
    id: "folder-id",
    uri: URI.file(resolve("project")).toString(),
  };
  const folder = parseWorkspaceIdentifier(serializedFolder);
  assert.ok(isSingleFolderWorkspaceIdentifier(folder));
  assert.equal(folder.uri.toString(), serializedFolder.uri);
  assert.deepEqual(
    serializeWorkspaceIdentifier(folder),
    serializedFolder,
  );

  assert.throws(
    () => parseWorkspaceIdentifier({ ...serializedFolder, unexpected: true }),
    /exactly/,
  );
  assert.throws(
    () =>
      parseWorkspaceIdentifier({
        id: "folder-id",
        uri: "https://example.com/project",
      }),
    /file scheme/,
  );
  assert.throws(
    () =>
      parseWorkspaceIdentifier({
        id: "folder-id",
        uri: `${serializedFolder.uri}?revision=1`,
      }),
    /query or fragment/,
  );
  assert.throws(
    () => parseWorkspaceIdentifier({ id: "" }),
    /non-empty/,
  );
});

test("workspaces main service exposes a window-owned identity through IPC", async () => {
  const workspace = await new WorkspacesMainService()
    .resolveStartupWorkspace({
      arguments: [],
      cwd: resolve("launch-root"),
    });
  const [route] = workspaceContextIpcRoutes(workspace);

  assert.equal(route.channel, "zeta:workspace:context:read");
  assert.equal(route.validate(undefined), undefined);
  assert.throws(() => route.validate({}), /does not accept parameters/);
  assert.deepEqual(
    await route.invoke(undefined),
    serializeWorkspaceIdentifier(UNKNOWN_EMPTY_WINDOW_WORKSPACE),
  );
});

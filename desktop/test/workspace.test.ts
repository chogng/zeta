import assert from "node:assert/strict";
import { resolve } from "node:path";
import test from "node:test";
import { URI } from "../src/base/common/uri.js";
import {
  EMPTY_WORKSPACE,
  parseWorkspaceContext,
  WorkbenchState,
} from "../src/platform/workspace/common/workspace.js";
import {
  parseWorkspaceLaunchArguments,
  resolveStartupWorkspace,
  WorkspaceLaunchTargetKind,
  type IWorkspacePathService,
  WorkspaceMainService,
  WorkspacePathKind,
  workspaceContextIpcRoutes,
} from "../src/platform/workspace/electron-main/workspaceMainService.js";
import {
  WindowKind,
  windowKindForWorkspace,
} from "../src/platform/window/common/window.js";

test("workspace launch arguments distinguish automatic and named targets", () => {
  assert.equal(parseWorkspaceLaunchArguments([]), undefined);
  assert.deepEqual(parseWorkspaceLaunchArguments(["project"]), {
    kind: WorkspaceLaunchTargetKind.Automatic,
    path: "project",
  });
  assert.deepEqual(parseWorkspaceLaunchArguments(["--folder", "project"]), {
    kind: WorkspaceLaunchTargetKind.Folder,
    path: "project",
  });
  assert.deepEqual(
    parseWorkspaceLaunchArguments(["--workspace=team.zeta-workspace"]),
    {
      kind: WorkspaceLaunchTargetKind.Workspace,
      path: "team.zeta-workspace",
    },
  );
  assert.deepEqual(parseWorkspaceLaunchArguments(["--", "-project"]), {
    kind: WorkspaceLaunchTargetKind.Automatic,
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

test("startup workspace resolves a canonical folder identity", async () => {
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

  const workspace = await resolveStartupWorkspace({
    arguments: ["project"],
    cwd,
    pathService,
  });

  assert.equal(requestedPath, resolve(cwd, "project"));
  assert.deepEqual(workspace, {
    state: WorkbenchState.FOLDER,
    uri: URI.file(canonicalPath).toString(),
    label: "project",
  });
  assert.equal(windowKindForWorkspace(workspace), WindowKind.Workspace);
});

test("startup workspace recognizes explicit workspace files", async () => {
  const canonicalPath = resolve("canonical", "team.zeta-workspace");
  const pathService: IWorkspacePathService = {
    async resolvePath() {
      return {
        kind: WorkspacePathKind.File,
        path: canonicalPath,
      };
    },
  };

  const workspace = await resolveStartupWorkspace({
    arguments: ["--workspace", "team.zeta-workspace"],
    cwd: resolve("launch-root"),
    pathService,
  });

  assert.deepEqual(workspace, {
    state: WorkbenchState.WORKSPACE,
    configUri: URI.file(canonicalPath).toString(),
    label: "team",
  });
});

test("startup workspace keeps loose files and missing targets empty", async () => {
  const filePath = resolve("canonical", "notes.txt");
  const pathService: IWorkspacePathService = {
    async resolvePath() {
      return {
        kind: WorkspacePathKind.File,
        path: filePath,
      };
    },
  };

  assert.deepEqual(
    await resolveStartupWorkspace({
      arguments: [],
      cwd: resolve("launch-root"),
      pathService,
    }),
    EMPTY_WORKSPACE,
  );
  const looseFileWorkspace = await resolveStartupWorkspace({
    arguments: ["notes.txt"],
    cwd: resolve("launch-root"),
    pathService,
  });
  assert.deepEqual(looseFileWorkspace, EMPTY_WORKSPACE);
  assert.equal(
    windowKindForWorkspace(looseFileWorkspace),
    WindowKind.Empty,
  );
  await assert.rejects(
    resolveStartupWorkspace({
      arguments: ["--folder", "notes.txt"],
      cwd: resolve("launch-root"),
      pathService,
    }),
    /not a directory/,
  );
});

test("workspace context validation rejects malformed renderer data", () => {
  const folder = {
    state: WorkbenchState.FOLDER,
    uri: URI.file(resolve("project")).toString(),
    label: "project",
  };
  assert.deepEqual(parseWorkspaceContext(folder), folder);
  assert.throws(
    () => parseWorkspaceContext({ ...folder, unexpected: true }),
    /exactly/,
  );
  assert.throws(
    () =>
      parseWorkspaceContext({
        state: WorkbenchState.FOLDER,
        uri: "https://example.com/project",
        label: "project",
      }),
    /file scheme/,
  );
  assert.throws(
    () =>
      parseWorkspaceContext({
        state: WorkbenchState.FOLDER,
        uri: `${folder.uri}?revision=1`,
        label: "project",
      }),
    /query or fragment/,
  );
});

test("workspace main service exposes its immutable context through IPC", async () => {
  const service = await WorkspaceMainService.create({
    arguments: [],
    cwd: resolve("launch-root"),
  });
  const [route] = workspaceContextIpcRoutes(service);

  assert.equal(route.channel, "zeta:workspace:context:read");
  assert.equal(route.validate(undefined), undefined);
  assert.throws(() => route.validate({}), /does not accept parameters/);
  assert.deepEqual(await route.invoke(undefined), EMPTY_WORKSPACE);
});

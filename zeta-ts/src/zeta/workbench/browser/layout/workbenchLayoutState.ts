import type { IStorageService } from "../../../platform/storage/common/storage.js";
import { StorageScope, StorageTarget } from "../../../platform/storage/common/storage.js";

const DEFAULT_SIDEBAR_WIDTH = 220;
const DEFAULT_AUXILIARYBAR_WIDTH = 380;
const DEFAULT_AGENT_SIDEBAR_WIDTH = 280;
const DEFAULT_PANEL_HEIGHT = 200;

/** The persisted, application-scoped portion of Workbench layout state. */
export interface WorkbenchLayoutState {
  readonly version: 3;
  readonly sidebar: {
    readonly width: number;
    readonly visible: boolean;
  };
  readonly auxiliarybar: {
    readonly width: number;
    readonly visible: boolean;
  };
  readonly agentSidebar: {
    readonly width: number;
    readonly visible: boolean;
  };
  readonly panel: {
    readonly height: number;
    readonly visible: boolean;
  };
}

export function createDefaultWorkbenchLayoutState(): WorkbenchLayoutState {
  return {
    version: 3,
    sidebar: {
      width: DEFAULT_SIDEBAR_WIDTH,
      visible: true,
    },
    auxiliarybar: {
      width: DEFAULT_AUXILIARYBAR_WIDTH,
      visible: true,
    },
    agentSidebar: {
      width: DEFAULT_AGENT_SIDEBAR_WIDTH,
      visible: false,
    },
    panel: {
      height: DEFAULT_PANEL_HEIGHT,
      visible: true,
    },
  };
}

export function parseWorkbenchLayoutState(
  value: unknown,
): WorkbenchLayoutState {
  if (
    !isRecord(value) ||
    !isHorizontalLayoutRegionState(value.sidebar) ||
    !isHorizontalLayoutRegionState(value.auxiliarybar)
  ) {
    throw new TypeError("Workbench layout state is invalid or unsupported");
  }
  let panel: { readonly height: number; readonly visible: boolean };
  let agentSidebar: { readonly width: number; readonly visible: boolean };
  if (value.version === 1) {
    panel = { height: DEFAULT_PANEL_HEIGHT, visible: true };
    agentSidebar = { width: DEFAULT_AGENT_SIDEBAR_WIDTH, visible: false };
  } else if (value.version === 2 && isVerticalLayoutRegionState(value.panel)) {
    panel = value.panel;
    agentSidebar = { width: DEFAULT_AGENT_SIDEBAR_WIDTH, visible: false };
  } else if (
    value.version === 3 &&
    isVerticalLayoutRegionState(value.panel) &&
    isHorizontalLayoutRegionState(value.agentSidebar)
  ) {
    panel = value.panel;
    agentSidebar = value.agentSidebar;
  } else {
    throw new TypeError("Workbench layout state is invalid or unsupported");
  }
  return {
    version: 3,
    sidebar: {
      width: value.sidebar.width,
      visible: value.sidebar.visible,
    },
    auxiliarybar: {
      width: value.auxiliarybar.width,
      visible: value.auxiliarybar.visible,
    },
    agentSidebar,
    panel: {
      height: panel.height,
      visible: panel.visible,
    },
  };
}

/** Bridges Workbench layout semantics to the generic scoped storage service. */
export class WorkbenchLayoutStateModel {
  constructor(
    private readonly storageService: IStorageService | undefined,
    private readonly defaults: WorkbenchLayoutState,
  ) {}

  get state(): WorkbenchLayoutState {
    const storage = this.storageService;
    if (!storage) return this.defaults;
    return {
      version: 3,
      sidebar: {
        width: storedDimension(
          storage.getNumber(
            WorkbenchLayoutStorageKeys.SIDEBAR_WIDTH.key,
            WorkbenchLayoutStorageKeys.SIDEBAR_WIDTH.scope,
          ),
          this.defaults.sidebar.width,
        ),
        visible: storage.getBoolean(
          WorkbenchLayoutStorageKeys.SIDEBAR_VISIBLE.key,
          WorkbenchLayoutStorageKeys.SIDEBAR_VISIBLE.scope,
          this.defaults.sidebar.visible,
        ),
      },
      auxiliarybar: {
        width: storedDimension(
          storage.getNumber(
            WorkbenchLayoutStorageKeys.AUXILIARYBAR_WIDTH.key,
            WorkbenchLayoutStorageKeys.AUXILIARYBAR_WIDTH.scope,
          ),
          this.defaults.auxiliarybar.width,
        ),
        visible: storage.getBoolean(
          WorkbenchLayoutStorageKeys.AUXILIARYBAR_VISIBLE.key,
          WorkbenchLayoutStorageKeys.AUXILIARYBAR_VISIBLE.scope,
          this.defaults.auxiliarybar.visible,
        ),
      },
      agentSidebar: {
        width: storedDimension(
          storage.getNumber(
            WorkbenchLayoutStorageKeys.AGENT_SIDEBAR_WIDTH.key,
            WorkbenchLayoutStorageKeys.AGENT_SIDEBAR_WIDTH.scope,
          ),
          this.defaults.agentSidebar.width,
        ),
        visible: storage.getBoolean(
          WorkbenchLayoutStorageKeys.AGENT_SIDEBAR_VISIBLE.key,
          WorkbenchLayoutStorageKeys.AGENT_SIDEBAR_VISIBLE.scope,
          this.defaults.agentSidebar.visible,
        ),
      },
      panel: {
        height: storedDimension(
          storage.getNumber(
            WorkbenchLayoutStorageKeys.PANEL_HEIGHT.key,
            WorkbenchLayoutStorageKeys.PANEL_HEIGHT.scope,
          ),
          this.defaults.panel.height,
        ),
        visible: storage.getBoolean(
          WorkbenchLayoutStorageKeys.PANEL_VISIBLE.key,
          WorkbenchLayoutStorageKeys.PANEL_VISIBLE.scope,
          this.defaults.panel.visible,
        ),
      },
    };
  }

  save(state: WorkbenchLayoutState): void {
    const storage = this.storageService;
    if (!storage) return;
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.SIDEBAR_WIDTH,
      state.sidebar.width,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.SIDEBAR_VISIBLE,
      state.sidebar.visible,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.AUXILIARYBAR_WIDTH,
      state.auxiliarybar.width,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.AUXILIARYBAR_VISIBLE,
      state.auxiliarybar.visible,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.AGENT_SIDEBAR_WIDTH,
      state.agentSidebar.width,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.AGENT_SIDEBAR_VISIBLE,
      state.agentSidebar.visible,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.PANEL_HEIGHT,
      state.panel.height,
    );
    storeLayoutValue(
      storage,
      WorkbenchLayoutStorageKeys.PANEL_VISIBLE,
      state.panel.visible,
    );
  }
}

interface WorkbenchLayoutStorageKey {
  readonly key: string;
  readonly scope: StorageScope;
  readonly target: StorageTarget;
}

const WorkbenchLayoutStorageKeys = {
  SIDEBAR_WIDTH: {
    key: "workbench.layout.sidebar.width",
    scope: StorageScope.PROFILE,
    target: StorageTarget.MACHINE,
  },
  SIDEBAR_VISIBLE: {
    key: "workbench.layout.sidebar.visible",
    scope: StorageScope.WORKSPACE,
    target: StorageTarget.MACHINE,
  },
  AUXILIARYBAR_WIDTH: {
    key: "workbench.layout.auxiliarybar.width",
    scope: StorageScope.PROFILE,
    target: StorageTarget.MACHINE,
  },
  AUXILIARYBAR_VISIBLE: {
    key: "workbench.layout.auxiliarybar.visible",
    scope: StorageScope.WORKSPACE,
    target: StorageTarget.MACHINE,
  },
  AGENT_SIDEBAR_WIDTH: {
    key: "workbench.layout.agentSidebar.width",
    scope: StorageScope.PROFILE,
    target: StorageTarget.MACHINE,
  },
  AGENT_SIDEBAR_VISIBLE: {
    key: "workbench.layout.agentSidebar.visible",
    scope: StorageScope.WORKSPACE,
    target: StorageTarget.MACHINE,
  },
  PANEL_HEIGHT: {
    key: "workbench.layout.panel.height",
    scope: StorageScope.PROFILE,
    target: StorageTarget.MACHINE,
  },
  PANEL_VISIBLE: {
    key: "workbench.layout.panel.visible",
    scope: StorageScope.WORKSPACE,
    target: StorageTarget.MACHINE,
  },
} as const satisfies Record<string, WorkbenchLayoutStorageKey>;

function storeLayoutValue(
  storage: IStorageService,
  key: WorkbenchLayoutStorageKey,
  value: number | boolean,
): void {
  storage.store(key.key, value, key.scope, key.target);
}

function storedDimension(value: number | undefined, fallback: number): number {
  return value !== undefined && value >= 0 ? value : fallback;
}

function isHorizontalLayoutRegionState(
  value: unknown,
): value is { readonly width: number; readonly visible: boolean } {
  return (
    isRecord(value) &&
    isLayoutDimension(value.width) &&
    typeof value.visible === "boolean"
  );
}

function isVerticalLayoutRegionState(
  value: unknown,
): value is { readonly height: number; readonly visible: boolean } {
  return (
    isRecord(value) &&
    isLayoutDimension(value.height) &&
    typeof value.visible === "boolean"
  );
}

function isLayoutDimension(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

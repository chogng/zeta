import { parseWorkbenchLayoutState, type WorkbenchLayoutState } from "./layout/workbenchLayoutState.js";

/**
 * Host-supplied Workbench composition profile.
 *
 * Workbench owns the layout/runtime contract, while the profile owns the
 * initial arrangement. Build-mode capability selection is intentionally
 * independent from this UI profile.
 */
export interface WorkbenchProfile {
  readonly id: string;
  readonly label: string;
  readonly layout: WorkbenchLayoutState;
  readonly composition: WorkbenchProfileComposition;
}

/** Initial view-container selection for each retained Workbench region. */
export interface WorkbenchProfileComposition {
  readonly sidebar: string;
  readonly auxiliarybar: string;
  readonly agentSidebar: string;
  readonly panel: string;
}

/** Validates the host-facing identity and layout contract before construction. */
export function validateWorkbenchProfile(profile: WorkbenchProfile): void {
  if (!profile || typeof profile !== "object") throw new TypeError("Workbench profile is required");
  if (typeof profile.id !== "string" || !/^[a-z][a-z0-9-]*$/.test(profile.id)) throw new TypeError("Workbench profile id must be a stable kebab-case identifier");
  if (typeof profile.label !== "string" || profile.label.trim().length === 0) throw new TypeError("Workbench profile label must not be empty");
  parseWorkbenchLayoutState(profile.layout);
  validateWorkbenchProfileComposition(profile.composition);
}

/** Creates an immutable UI profile after validating its Workbench defaults. */
export function createWorkbenchProfile(profile: WorkbenchProfile): WorkbenchProfile {
  validateWorkbenchProfile(profile);
  const layout = parseWorkbenchLayoutState(profile.layout);
  return Object.freeze({
    id: profile.id,
    label: profile.label,
    layout: Object.freeze({
      version: 3 as const,
      sidebar: Object.freeze({ ...layout.sidebar }),
      auxiliarybar: Object.freeze({ ...layout.auxiliarybar }),
      agentSidebar: Object.freeze({ ...layout.agentSidebar }),
      panel: Object.freeze({ ...layout.panel }),
    }),
    composition: Object.freeze({ ...profile.composition }),
  });
}

function validateWorkbenchProfileComposition(
  composition: WorkbenchProfileComposition,
): void {
  if (!composition || typeof composition !== "object") throw new TypeError("Workbench profile composition is required");
  for (const [region, containerId] of Object.entries(composition)) {
    if (typeof containerId !== "string" || containerId.length === 0) throw new TypeError(`Workbench profile ${region} composition must name a view container`);
  }
  if (Object.keys(composition).length !== 4 || !("sidebar" in composition) || !("auxiliarybar" in composition) || !("agentSidebar" in composition) || !("panel" in composition)) {
    throw new TypeError("Workbench profile composition must define every Workbench region");
  }
}

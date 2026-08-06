import { parseWorkbenchLayoutState, type WorkbenchLayoutState } from "./layout/workbenchLayoutState.js";

/**
 * Product-owned Workbench composition profile.
 *
 * The product entry supplies one profile before the shared Workbench starts.
 * Workbench owns the layout/runtime contract, while the profile owns the
 * initial arrangement appropriate for Code, Academic, or a combined build.
 */
export interface WorkbenchSession {
  readonly id: string;
  readonly label: string;
  readonly layout: WorkbenchLayoutState;
  readonly composition: WorkbenchSessionComposition;
}

/** Initial view-container selection for each retained Workbench region. */
export interface WorkbenchSessionComposition {
  readonly sidebar: string;
  readonly auxiliarybar: string;
  readonly agentSidebar: string;
  readonly panel: string;
}

/** Validates the host-facing identity and layout contract before construction. */
export function validateWorkbenchSession(session: WorkbenchSession): void {
  if (!session || typeof session !== "object") throw new TypeError("Workbench session is required");
  if (typeof session.id !== "string" || !/^[a-z][a-z0-9-]*$/.test(session.id)) throw new TypeError("Workbench session id must be a stable kebab-case identifier");
  if (typeof session.label !== "string" || session.label.trim().length === 0) throw new TypeError("Workbench session label must not be empty");
  parseWorkbenchLayoutState(session.layout);
  validateWorkbenchSessionComposition(session.composition);
}

/** Creates an immutable product session profile after validating its Workbench defaults. */
export function createWorkbenchSession(session: WorkbenchSession): WorkbenchSession {
  validateWorkbenchSession(session);
  const layout = parseWorkbenchLayoutState(session.layout);
  return Object.freeze({
    id: session.id,
    label: session.label,
    layout: Object.freeze({
      version: 3 as const,
      sidebar: Object.freeze({ ...layout.sidebar }),
      auxiliarybar: Object.freeze({ ...layout.auxiliarybar }),
      agentSidebar: Object.freeze({ ...layout.agentSidebar }),
      panel: Object.freeze({ ...layout.panel }),
    }),
    composition: Object.freeze({ ...session.composition }),
  });
}

function validateWorkbenchSessionComposition(
  composition: WorkbenchSessionComposition,
): void {
  if (!composition || typeof composition !== "object") throw new TypeError("Workbench session composition is required");
  for (const [region, containerId] of Object.entries(composition)) {
    if (typeof containerId !== "string" || containerId.length === 0) throw new TypeError(`Workbench session ${region} composition must name a view container`);
  }
  if (Object.keys(composition).length !== 4 || !("sidebar" in composition) || !("auxiliarybar" in composition) || !("agentSidebar" in composition) || !("panel" in composition)) {
    throw new TypeError("Workbench session composition must define every Workbench region");
  }
}

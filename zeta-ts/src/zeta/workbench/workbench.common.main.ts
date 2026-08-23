/**
 * Shared Workbench registrations loaded by every renderer host.
 *
 * Host-specific services and contributions belong in `workbench.web.main.ts`
 * or `workbench.desktop.main.ts`, while product entries remain responsible
 * for selecting their editor contributions.
 */
import "./browser/workbench.contribution.js";

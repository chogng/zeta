import { register } from '../../../../base/common/icon.js';
import { lxiconsLibrary } from '../../../../base/common/lxiconsLibrary.js';

export const foldingExpandedIcon = register('folding-expanded', lxiconsLibrary.chevronDown);
export const foldingCollapsedIcon = register('folding-collapsed', lxiconsLibrary.chevronRight);
export const foldingManualCollapsedIcon = register('folding-manual-collapsed', foldingCollapsedIcon);
export const foldingManualExpandedIcon = register('folding-manual-expanded', foldingExpandedIcon);

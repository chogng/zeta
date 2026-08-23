/** Stable application identity shared by every built-in Desktop mode. */
export interface DesktopApplicationConfiguration {
	readonly name: string;
	readonly applicationId: string;
	readonly userDataFolderName: string;
	readonly rendererDirectory: string;
}

export const ZetaDesktopApplication: DesktopApplicationConfiguration = {
	name: 'Zeta',
	applicationId: 'com.zeta.desktop',
	userDataFolderName: 'Zeta',
	rendererDirectory: 'zeta',
};

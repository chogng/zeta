/** Selects one visual side when a model position has multiple rendered locations. */
export enum PositionAffinity {
	Left = 0,
	Right = 1,
	None = 2,
	LeftOfInjectedText = 3,
	RightOfInjectedText = 4,
}

/** Describes a change to the language associated with a text model. */
export interface IModelLanguageChangedEvent {
	readonly oldLanguage: string;
	readonly newLanguage: string;
	readonly source: string;
}

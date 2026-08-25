export interface PreferencesSearchTarget {
	readonly title: string;
	readonly description: string;
	readonly keywords?: readonly string[];
}

/** Normalizes one Preferences query and matches it against searchable setting metadata. */
export class PreferencesSearchQuery {
	public readonly text: string;
	private readonly terms: readonly string[];

	constructor(value: string) {
		this.text = value
			.replace(/[":]/gu, ' ')
			.replace(/\s+/gu, ' ')
			.trim()
			.toLocaleLowerCase();
		this.terms = this.text ? this.text.split(' ') : [];
	}

	public get isEmpty(): boolean {
		return this.terms.length === 0;
	}

	public matches(target: PreferencesSearchTarget): boolean {
		if (this.isEmpty) return true;
		const searchableText = [target.title, target.description, ...(target.keywords ?? [])]
			.join(' ')
			.toLocaleLowerCase();
		return this.terms.every(term => searchableText.includes(term));
	}
}

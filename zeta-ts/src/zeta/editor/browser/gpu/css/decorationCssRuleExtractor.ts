import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import './media/decorationCssRuleExtractor.css';

export class DecorationCssRuleExtractor extends Disposable {
	private readonly container: HTMLDivElement;
	private readonly dummyElement: HTMLSpanElement;
	private readonly ruleCache = new Map<string, CSSStyleRule[]>();
	private readonly cssVariableCache = new Map<string, string>();

	constructor(private readonly ownerDocument: Document) {
		super();
		this.container = ownerDocument.createElement('div');
		this.container.className = 'stanza-decoration-css-rule-extractor';
		this.dummyElement = ownerDocument.createElement('span');
		this.container.append(this.dummyElement);
		this._register(toDisposable(() => this.container.remove()));
	}

	public getStyleRules(host: HTMLElement, decorationClassName: string): readonly CSSStyleRule[] {
		const existing = this.ruleCache.get(decorationClassName);
		if (existing) return existing;
		this.dummyElement.className = decorationClassName;
		host.append(this.container);
		const classNames = decorationClassName.split(' ').filter(Boolean);
		const rules: CSSStyleRule[] = [];
		for (const stylesheet of this.ownerDocument.styleSheets) this.collectMatchingRules(stylesheet.cssRules, classNames, rules);
		this.container.remove();
		this.ruleCache.set(decorationClassName, rules);
		return rules;
	}

	public resolveCssVariable(canvas: HTMLCanvasElement, variableName: string): string {
		const existing = this.cssVariableCache.get(variableName);
		if (existing !== undefined) return existing;
		canvas.parentElement?.append(this.container);
		const value = this.ownerDocument.defaultView?.getComputedStyle(this.container).getPropertyValue(variableName).trim() ?? '';
		this.container.remove();
		this.cssVariableCache.set(variableName, value);
		return value;
	}

	public clear(): void {
		this.ruleCache.clear();
		this.cssVariableCache.clear();
	}

	private collectMatchingRules(ruleList: CSSRuleList, classNames: readonly string[], result: CSSStyleRule[]): void {
		for (const rule of ruleList) {
			if (rule instanceof this.ownerDocument.defaultView!.CSSImportRule && rule.styleSheet) {
				this.collectMatchingRules(rule.styleSheet.cssRules, classNames, result);
				continue;
			}
			if (!(rule instanceof this.ownerDocument.defaultView!.CSSStyleRule)) continue;
			if (classNames.some(className => rule.selectorText.includes(`.${className}`))) result.push(rule);
			if (rule.cssRules?.length) this.collectMatchingRules(rule.cssRules, classNames, result);
		}
	}
}

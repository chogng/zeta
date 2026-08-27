import { Emitter } from "../../../base/common/event.js";
import { Disposable } from "../../../base/common/lifecycle.js";
import type { URI } from "../../../base/common/uri.js";
import { isDarkColorScheme } from "../common/theme.js";
import type { IThemeService } from "../common/themeService.js";
import type { IFileIconThemeService } from "./fileIconThemeService.js";
import setiThemeData from "./media/seti/vs-seti-icon-theme.json" with {
	type: "json",
};
import "./setiFileIconTheme.css";

type SetiFileIconAssociations =
	| typeof setiThemeData
	| typeof setiThemeData.light;

type SetiIconDefinition = {
	readonly fontCharacter?: string;
	readonly fontColor?: string;
	readonly fontSize?: string;
};

const setiIconDefinitions: Readonly<Record<string, SetiIconDefinition>> =
	setiThemeData.iconDefinitions;

const LANGUAGE_ID_BY_EXTENSION = new Map<string, string>([
	["bash", "shellscript"],
	["cc", "cpp"],
	["cjs", "javascript"],
	["clj", "clojure"],
	["cljs", "clojure"],
	["coffee", "coffeescript"],
	["cs", "csharp"],
	["cxx", "cpp"],
	["fs", "fsharp"],
	["fsx", "fsharp"],
	["h", "c"],
	["hh", "cpp"],
	["hpp", "cpp"],
	["hs", "haskell"],
	["js", "javascript"],
	["jsx", "javascriptreact"],
	["kt", "kotlin"],
	["kts", "kotlin"],
	["md", "markdown"],
	["mjs", "javascript"],
	["pl", "perl"],
	["pm", "perl"],
	["ps1", "powershell"],
	["py", "python"],
	["rb", "ruby"],
	["rs", "rust"],
	["sh", "shellscript"],
	["ts", "typescript"],
	["tsx", "typescriptreact"],
	["yml", "yaml"],
	["zsh", "shellscript"],
]);

/**
 * Built-in Seti file icon theme generated from `jesseweed/seti-ui`.
 */
export class SetiFileIconThemeService extends Disposable
	implements IFileIconThemeService {
	private readonly _onDidFileIconThemeChange = this._register(new Emitter<void>());
	private readonly themeService: IThemeService;

	readonly onDidFileIconThemeChange =
		this._onDidFileIconThemeChange.event;

	constructor(themeService: IThemeService) {
		super();
		this.themeService = themeService;
		this._register(themeService.onDidColorThemeChange(() => {
			this._onDidFileIconThemeChange.fire();
		}));
	}

	renderFileIcon(resource: URI, container: HTMLElement): void {
		const definition = this.resolveDefinition(fileName(resource));
		container.classList.remove("zeta-seti-file-icon");
		container.classList.add("zeta-seti-file-icon");
		container.style.color = "";
		container.style.fontSize = "";
		container.textContent = decodeFontCharacter(
			definition?.fontCharacter ?? "",
		);
		if (definition?.fontColor) {
			container.style.color = definition.fontColor;
		}
		if (definition?.fontSize) {
			container.style.fontSize = definition.fontSize;
		}
	}

	private resolveDefinition(fileName: string): SetiIconDefinition | undefined {
		const useLightAssociations = !isDarkColorScheme(
			this.themeService.getColorTheme().colorScheme,
		);
		const iconId = useLightAssociations
			? resolveSpecificIconId(setiThemeData.light, fileName) ??
				resolveSpecificIconId(setiThemeData, fileName) ??
				setiThemeData.light.file
			: resolveSpecificIconId(setiThemeData, fileName) ?? setiThemeData.file;
		return iconId
			? setiIconDefinitions[iconId]
			: undefined;
	}
}

function resolveSpecificIconId(
	associations: SetiFileIconAssociations,
	fileName: string,
): string | undefined {
	const normalizedName = fileName.toLowerCase();
	const fileNames: Readonly<Record<string, string>> =
		associations.fileNames;
	const fileExtensions: Readonly<Record<string, string>> =
		associations.fileExtensions;
	const languageIds: Readonly<Record<string, string>> =
		associations.languageIds;
	const exactMatch = fileNames[normalizedName];
	if (exactMatch) return exactMatch;

	for (const extension of extensionCandidates(normalizedName)) {
		const extensionMatch = fileExtensions[extension];
		if (extensionMatch) return extensionMatch;
	}

	const languageId = languageIdForFileName(
		normalizedName,
		languageIds,
	);
	return languageId
		? languageIds[languageId]
		: undefined;
}

function extensionCandidates(fileName: string): readonly string[] {
	const segments = fileName.split(".");
	if (segments.length < 2) return [fileName];
	const extensions: string[] = [];
	for (let index = 1; index < segments.length; index += 1) {
		extensions.push(segments.slice(index).join("."));
	}
	return extensions;
}

function languageIdForFileName(
	fileName: string,
	languageIds: Readonly<Record<string, string>>,
): string | undefined {
	const extension = fileName.slice(fileName.lastIndexOf(".") + 1);
	const mapped = LANGUAGE_ID_BY_EXTENSION.get(extension);
	if (mapped) return mapped;
	return languageIds[extension] ? extension : undefined;
}

function fileName(resource: URI): string {
	const separator = resource.path.lastIndexOf("/");
	return decodeURIComponent(resource.path.slice(separator + 1));
}

function decodeFontCharacter(value: string): string {
	const cssEscape = /^\\([0-9a-f]{1,6})$/i.exec(value);
	return cssEscape
		? String.fromCodePoint(Number.parseInt(cssEscape[1], 16))
		: value;
}

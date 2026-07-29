import { DisposableOwner, ResettableDisposableGroup } from "../../base/common/lifecycle.js";
import { parseUserColorTheme } from "../../platform/theme/common/userColorTheme.js";
import { type IUserThemeFileList, type IUserThemeFilesApi, validateUserThemeFileList } from "../../platform/theme/common/userThemeFiles.js";
import { WorkbenchThemesRegistry } from "../common/theme.js";
import { type IUserThemeDeleteResult, type IUserThemeLoadIssue, type IUserThemeSaveResult, type IUserThemeService, type IUserThemeSource } from "../common/userThemes.js";

/** Electron-backed user theme collection with isolated registration and writes. */
export class ElectronUserThemeService extends DisposableOwner implements IUserThemeService {
  readonly available = true;
  readonly #api: IUserThemeFilesApi;
  readonly #registrations = this.own(new ResettableDisposableGroup());
  readonly #contents = new Map<string, string>();
  #directory: string | undefined;
  #sources: readonly IUserThemeSource[] = [];
  #issues: readonly IUserThemeLoadIssue[] = [];

  constructor(api: IUserThemeFilesApi) {
    super();
    this.#api = api;
  }

  get directory(): string | undefined {
    return this.#directory;
  }

  get issues(): readonly IUserThemeLoadIssue[] {
    return this.#issues;
  }

  sourceFor(themeId: string): IUserThemeSource | undefined {
    return this.#sources.find(({ id }) => id === themeId);
  }

  getSource(themeId: string): string | undefined {
    return this.#contents.get(themeId);
  }

  async delete(themeId: string): Promise<IUserThemeDeleteResult> {
    const existing = this.sourceFor(themeId);
    const theme = WorkbenchThemesRegistry.getColorTheme(themeId);
    if (!existing || !theme) throw new Error(`User theme is not loaded: ${themeId}`);
    this.#apply(validateUserThemeFileList(await this.#api.delete({
      file: existing.file,
      themeId,
    })));
    return { colorScheme: theme.colorScheme, file: existing.file };
  }

  async reload(): Promise<void> {
    try {
      this.#apply(validateUserThemeFileList(await this.#api.list()));
    } catch (error) {
      this.#setStatus(this.#directory, this.#sources, [{
        file: "themes",
        message: errorMessage(error, "Unable to discover user themes"),
      }]);
    }
  }

  async save(themeId: string, source: string): Promise<IUserThemeSaveResult> {
    const existing = this.sourceFor(themeId);
    if (!existing) throw new Error(`User theme is not loaded: ${themeId}`);
    const theme = parseUserColorTheme(source);
    if (theme.id !== themeId) throw new Error("Change the theme id only when using Save As");
    this.#apply(validateUserThemeFileList(await this.#api.write({
      content: source,
      file: existing.file,
      operation: "replace",
    })));
    return this.#savedResult(themeId);
  }

  async saveAs(source: string): Promise<IUserThemeSaveResult> {
    const theme = parseUserColorTheme(source);
    if (WorkbenchThemesRegistry.getColorTheme(theme.id)) throw new Error(`Theme id is already in use: ${theme.id}`);
    const file = `${theme.id}.json`;
    this.#apply(validateUserThemeFileList(await this.#api.write({
      content: source,
      file,
      operation: "create",
    })));
    return this.#savedResult(theme.id);
  }

  #apply(list: IUserThemeFileList): void {
    this.#registrations.clear();
    const sources: IUserThemeSource[] = [];
    const contents = new Map<string, string>();
    const issues: IUserThemeLoadIssue[] = [];
    for (const file of list.files) {
      if (file.error) {
        issues.push({ file: file.name, message: file.error });
        continue;
      }
      try {
        const theme = parseUserColorTheme(file.content!);
        this.#registrations.add(WorkbenchThemesRegistry.registerColorTheme(theme));
        sources.push({ id: theme.id, file: file.name });
        contents.set(theme.id, file.content!);
      } catch (error) {
        issues.push({
          file: file.name,
          message: errorMessage(error, "Unknown theme loading error"),
        });
      }
    }
    this.#contents.clear();
    for (const [id, content] of contents) this.#contents.set(id, content);
    this.#setStatus(list.directory, sources, issues);
  }

  #savedResult(themeId: string): IUserThemeSaveResult {
    const source = this.sourceFor(themeId);
    const theme = WorkbenchThemesRegistry.getColorTheme(themeId);
    if (!source || !theme) throw new Error(`Saved user theme could not be reloaded: ${themeId}`);
    return { file: source.file, theme };
  }

  #setStatus(directory: string | undefined, sources: readonly IUserThemeSource[], issues: readonly IUserThemeLoadIssue[]): void {
    this.#directory = directory;
    this.#sources = Object.freeze([...sources]);
    this.#issues = Object.freeze([...issues]);
  }
}

/** Loads the initial user theme collection before configuration is resolved. */
export async function loadUserThemes(api: IUserThemeFilesApi): Promise<ElectronUserThemeService> {
  const service = new ElectronUserThemeService(api);
  await service.reload();
  return service;
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

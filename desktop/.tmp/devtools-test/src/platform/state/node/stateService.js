import { mkdir, readFile, rename, writeFile, } from "node:fs/promises";
import { dirname } from "node:path";
function isRecord(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
function isFileNotFound(error) {
    return isRecord(error) && error.code === "ENOENT";
}
/**
 * Stores small main-process state in one JSON file and serializes writes.
 *
 * Each flush writes a temporary sibling file before replacing the destination,
 * preventing an interrupted write from leaving a partially written JSON file.
 */
export class StateService {
    #filePath;
    #temporaryFilePath;
    #items = Object.create(null);
    #lastSavedContents = "";
    #writeQueue = Promise.resolve();
    #closing;
    #closed = false;
    constructor(filePath) {
        this.#filePath = filePath;
        this.#temporaryFilePath = `${filePath}.${process.pid}.tmp`;
    }
    /** Opens a state file, treating a missing or malformed file as empty state. */
    static async create(filePath) {
        const service = new StateService(filePath);
        await service.#load();
        return service;
    }
    async #load() {
        let contents;
        try {
            contents = await readFile(this.#filePath, "utf8");
        }
        catch (error) {
            if (isFileNotFound(error)) {
                return;
            }
            throw error;
        }
        try {
            const parsed = JSON.parse(contents);
            if (isRecord(parsed)) {
                this.#items = parsed;
                this.#lastSavedContents = contents;
            }
        }
        catch (error) {
            if (!(error instanceof SyntaxError)) {
                throw error;
            }
        }
    }
    getItem(key) {
        return this.#items[key];
    }
    setItem(key, value) {
        this.#assertOpen();
        if (value === undefined) {
            delete this.#items[key];
        }
        else {
            this.#items[key] = value;
        }
    }
    removeItem(key) {
        this.#assertOpen();
        delete this.#items[key];
    }
    flush() {
        const contents = JSON.stringify(this.#items, null, 2);
        const write = this.#writeQueue
            .catch(() => undefined)
            .then(async () => {
            if (contents === this.#lastSavedContents) {
                return;
            }
            await mkdir(dirname(this.#filePath), { recursive: true });
            await writeFile(this.#temporaryFilePath, contents, "utf8");
            await rename(this.#temporaryFilePath, this.#filePath);
            this.#lastSavedContents = contents;
        });
        this.#writeQueue = write;
        return write;
    }
    close() {
        if (!this.#closing) {
            this.#closed = true;
            this.#closing = this.flush();
        }
        return this.#closing;
    }
    #assertOpen() {
        if (this.#closed) {
            throw new Error("State service is closed");
        }
    }
}

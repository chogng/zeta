import { DisposableOwner, } from "../../../base/common/lifecycle.js";
import { RevisionedJsonFile, } from "../../storage/node/revisionedJsonFile.js";
import { KEYBINDINGS_RESOURCE_READ_CHANNEL, KEYBINDINGS_RESOURCE_UPDATE_CHANNEL, validateKeybindingsResource, validateKeybindingsResourceRead, validateKeybindingsResourceUpdateRequest, } from "../common/keybindingsResource.js";
/**
 * Owns the active profile's `keybindings.json` in Electron main.
 */
export class KeybindingsResourceMainService extends DisposableOwner {
    #resource;
    constructor(resource) {
        super();
        this.#resource = this.own(resource);
    }
    static async create(options) {
        const resource = await RevisionedJsonFile.create({
            filePath: options.filePath,
            defaultValue: () => [],
            validate: validateKeybindingsResource,
            label: "Keybindings resource",
            onError: options.onError,
        });
        return new KeybindingsResourceMainService(resource);
    }
    get onDidChange() {
        return (listener) => this.#resource.onDidChange((snapshot) => listener({
            revision: snapshot.revision,
            bindings: snapshot.value,
        }));
    }
    read() {
        const snapshot = this.#resource.read();
        return {
            revision: snapshot.revision,
            bindings: snapshot.value,
        };
    }
    async update(request) {
        const snapshot = await this.#resource.update(request.expectedRevision, request.bindings);
        return {
            revision: snapshot.revision,
            bindings: snapshot.value,
        };
    }
    async close() {
        await this.#resource.close();
        this.dispose();
    }
}
export function keybindingsResourceIpcRoutes(service) {
    return [
        {
            channel: KEYBINDINGS_RESOURCE_READ_CHANNEL,
            validate: validateKeybindingsResourceRead,
            invoke: () => service.read(),
        },
        {
            channel: KEYBINDINGS_RESOURCE_UPDATE_CHANNEL,
            validate: validateKeybindingsResourceUpdateRequest,
            invoke: (request) => service.update(request),
        },
    ];
}

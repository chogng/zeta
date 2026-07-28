import { DisposableOwner, } from "../../../base/common/lifecycle.js";
import { RevisionedJsonFile, } from "../../storage/node/revisionedJsonFile.js";
import { CONFIGURATION_READ_CHANNEL, CONFIGURATION_UPDATE_CHANNEL, emptyConfigurationDocument, validateConfigurationDocument, validateConfigurationRead, validateConfigurationUpdateRequest, } from "../common/configuration.js";
/**
 * Owns the Desktop configuration resource in the Electron main process.
 */
export class ConfigurationMainService extends DisposableOwner {
    #resource;
    constructor(resource) {
        super();
        this.#resource = this.own(resource);
    }
    static async create(options) {
        const resource = await RevisionedJsonFile.create({
            filePath: options.filePath,
            defaultValue: emptyConfigurationDocument,
            validate: validateConfigurationDocument,
            label: "Configuration",
            onError: options.onError,
        });
        return new ConfigurationMainService(resource);
    }
    get onDidChange() {
        return (listener) => this.#resource.onDidChange((snapshot) => listener({
            revision: snapshot.revision,
            document: snapshot.value,
        }));
    }
    read() {
        const snapshot = this.#resource.read();
        return {
            revision: snapshot.revision,
            document: snapshot.value,
        };
    }
    async update(request) {
        const snapshot = await this.#resource.update(request.expectedRevision, request.document);
        return {
            revision: snapshot.revision,
            document: snapshot.value,
        };
    }
    async close() {
        await this.#resource.close();
        this.dispose();
    }
}
export function configurationIpcRoutes(service) {
    return [
        {
            channel: CONFIGURATION_READ_CHANNEL,
            validate: validateConfigurationRead,
            invoke: () => service.read(),
        },
        {
            channel: CONFIGURATION_UPDATE_CHANNEL,
            validate: validateConfigurationUpdateRequest,
            invoke: (request) => service.update(request),
        },
    ];
}

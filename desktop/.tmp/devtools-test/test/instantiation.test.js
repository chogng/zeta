import assert from "node:assert/strict";
import test from "node:test";
import { createServiceIdentifier, InstantiationService, ServiceCollection, SyncDescriptor, } from "../src/platform/instantiation/common/instantiation.js";
test("instantiation resolves descriptor arguments in contract order", () => {
    const serviceId = createServiceIdentifier("test.message");
    const services = new ServiceCollection();
    services.set(serviceId, "service");
    const instantiation = new InstantiationService(services);
    const descriptor = new SyncDescriptor(TestContribution, {
        staticArguments: ["static"],
        serviceDependencies: [serviceId],
    });
    const contribution = instantiation.createInstance(descriptor, "dynamic");
    assert.deepEqual(contribution.arguments, [
        "static",
        "dynamic",
        "service",
    ]);
});
test("invocation accessors cannot escape their call", () => {
    const serviceId = createServiceIdentifier("test.value");
    const services = new ServiceCollection();
    services.set(serviceId, "ready");
    const instantiation = new InstantiationService(services);
    let escapedAccessor;
    const result = instantiation.invokeFunction((accessor, suffix) => {
        escapedAccessor = accessor;
        return `${accessor.get(serviceId)}:${suffix}`;
    }, "done");
    assert.equal(result, "ready:done");
    assert.throws(() => escapedAccessor?.get(serviceId), /only valid during invocation/);
});
class TestContribution {
    arguments;
    constructor(staticArgument, dynamicArgument, service) {
        this.arguments = [staticArgument, dynamicArgument, service];
    }
}

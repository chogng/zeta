import assert from "node:assert/strict";
import test from "node:test";
import {
  createServiceIdentifier,
  InstantiationService,
  ServiceCollection,
  SyncDescriptor,
  type ServicesAccessor,
} from "../../../../platform/instantiation/common/instantiation.js";

test("instantiation resolves descriptor arguments in contract order", () => {
  const serviceId = createServiceIdentifier<string>("test.message");
  const services = new ServiceCollection();
  services.set(serviceId, "service");
  const instantiation = new InstantiationService(services);
  const descriptor = new SyncDescriptor(TestContribution, {
    staticArguments: ["static"],
    serviceDependencies: [serviceId],
  });

  const contribution = instantiation.createInstance(
    descriptor,
    "dynamic",
  );

  assert.deepEqual(contribution.arguments, [
    "static",
    "dynamic",
    "service",
  ]);
});

test("invocation accessors cannot escape their call", () => {
  const serviceId = createServiceIdentifier<string>("test.value");
  const services = new ServiceCollection();
  services.set(serviceId, "ready");
  const instantiation = new InstantiationService(services);
  let escapedAccessor: ServicesAccessor | undefined;

  const result = instantiation.invokeFunction((accessor, suffix) => {
    escapedAccessor = accessor;
    return `${accessor.get(serviceId)}:${suffix}`;
  }, "done");

  assert.equal(result, "ready:done");
  assert.throws(
    () => escapedAccessor?.get(serviceId),
    /only valid during invocation/,
  );
});

class TestContribution {
  readonly arguments: readonly string[];

  constructor(
    staticArgument: string,
    dynamicArgument: string,
    service: string,
  ) {
    this.arguments = [staticArgument, dynamicArgument, service];
  }
}

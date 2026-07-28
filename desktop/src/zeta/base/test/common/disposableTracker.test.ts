import { strict as assert } from "node:assert";
import test from "node:test";
import {
  DisposableStore,
  DisposableOwner,
  toDisposable,
} from "../../common/lifecycle.js";
import {
  DisposableTracker,
  installDisposableTracker,
} from "../../common/disposableTracker.js";

test("DisposableTracker reports an unowned disposable until it is disposed", () => {
  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);
  const resource = toDisposable(() => {});

  const [leak] = tracker.leaks();
  assert.equal(leak?.label, "toDisposable");
  assert.equal(leak?.ownerLabel, undefined);
  assert.match(leak?.createdAt ?? "", /disposableTracker\.test/);
  assert.throws(() => tracker.assertNoLeaks(), /1 undisposed disposable/);

  resource.dispose();
  tracker.assertNoLeaks();
});

test("DisposableTracker records ownership and closes the complete subtree", () => {
  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);
  const store = new DisposableStore();
  store.add(toDisposable(() => {}));

  const child = tracker.leaks().find((leak) => leak.label === "toDisposable");
  assert.equal(child?.ownerLabel, "DisposableStore");

  store.dispose();
  tracker.assertNoLeaks();
});

test("DisposableTracker follows DisposableOwner through its internal store", () => {
  class Owner extends DisposableOwner {
    take(resource: Disposable): void {
      this.own(resource);
    }
  }

  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);
  const owner = new Owner();
  owner.take(toDisposable(() => {}));

  const store = tracker.leaks().find((leak) =>
    leak.label === "DisposableStore"
  );
  assert.equal(store?.ownerLabel, "Owner");

  owner.dispose();
  tracker.assertNoLeaks();
});

test("DisposableTracker rejects multiple owners before ownership transfers", () => {
  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);
  const first = new DisposableStore();
  const second = new DisposableStore();
  const resource = first.add(toDisposable(() => {}));

  assert.throws(
    () => second.add(resource),
    /already belongs to DisposableStore/,
  );

  first.dispose();
  second.dispose();
  tracker.assertNoLeaks();
});

test("DisposableTracker rejects ownership cycles", () => {
  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);
  const parent = new DisposableStore();
  const child = parent.add(new DisposableStore());

  assert.throws(
    () => child.add(parent),
    /ownership cannot contain a cycle/,
  );

  parent.dispose();
  tracker.assertNoLeaks();
});

test("DisposableTracker closes ownership records even when cleanup throws", () => {
  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);
  const store = new DisposableStore();
  store.add(toDisposable(() => {
    throw new Error("cleanup failed");
  }));

  assert.throws(() => store.dispose(), /cleanup failed/);
  tracker.assertNoLeaks();
});

test("tracking is disabled outside an installed development scope", () => {
  const tracker = new DisposableTracker();
  const resource = toDisposable(() => {});

  assert.equal(tracker.leaks().length, 0);
  resource.dispose();
});

test("only one DisposableTracker can be installed in a JavaScript realm", () => {
  const tracker = new DisposableTracker();
  using installation = installDisposableTracker(tracker);

  assert.throws(
    () => installDisposableTracker(new DisposableTracker()),
    /already installed/,
  );
});

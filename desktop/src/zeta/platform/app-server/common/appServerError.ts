/** Transport-independent error returned by an App Server request. */
export class AppServerRemoteError extends Error {
  constructor(readonly code: number, readonly errorName: string, readonly data: null) {
    super(errorName);
    this.name = "AppServerRemoteError";
  }
}

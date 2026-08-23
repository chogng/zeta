/** Editor activation preferences shared by resource-navigation surfaces. */
export interface EditorActivationOptions {
  /** Keeps the opened resource as a durable tab instead of a replaceable preview. */
  readonly pinned?: boolean;
  /** Leaves DOM focus with the navigation surface that requested the open. */
  readonly preserveFocus?: boolean;
}

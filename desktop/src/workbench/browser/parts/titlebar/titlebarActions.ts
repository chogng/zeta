export interface TitlebarAction {
  id: string;
  label: string;
  title?: string;
  enabled?: boolean;
  run(): void | Promise<void>;
}

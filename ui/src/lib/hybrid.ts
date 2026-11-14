import { invoke } from "@tauri-apps/api/core";
import type { Service } from "./api";

// Serialize mount, resize and unmount operations. A slow login must never
// resurrect a native view over Settings or a newly selected page.
let pending: Promise<unknown> = Promise.resolve();
export function browserCommand<T = void>(command: string, args: Record<string, unknown>): Promise<T> {
  const next = pending.then(() => invoke<T>(command, args));
  pending = next.catch(() => {});
  return next;
}
export interface BrowserStatus {
  site: Service;
  loading: boolean;
  error: string | null;
  downloaded: boolean;
}

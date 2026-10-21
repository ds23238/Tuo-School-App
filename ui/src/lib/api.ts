import { invoke } from "@tauri-apps/api/core";

// Spiegelt die Typen aus crates/core.

export type Service = "moodle" | "untis";

export interface Settings {
  /** `null` = Systemsprache. */
  locale: string | null;
  syncDir: string;
}

export interface Account {
  service: Service;
  baseUrl: string;
  username: string;
  displayName: string | null;
  /** Unix-Zeit in Sekunden. */
  createdAt: number;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listAccounts: () => invoke<Account[]>("list_accounts"),
  removeAccount: (service: Service) => invoke<void>("remove_account", { service }),
  checkKeyring: () => invoke<void>("check_keyring"),
};

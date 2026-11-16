import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as unknown as Record<string, any>;
    const callbacks = new Map<number, (value: unknown) => void>();
    const listeners = new Map<string, number[]>();
    let next = 1;
    w.calls = [];
    w.failBrowser = false;
    w.savedPasswordAvailable = false;
    w.settings = JSON.parse(sessionStorage.getItem("mock.settings") ?? "null") ?? { locale: "de", theme: "light", subjectColors: {}, syncDir: "/tmp/sync", syncEnabled: true, syncIntervalMinutes: 30, notifications: true, courseDisplay: "web", moodleMessagesDisplay: "native", untisMessagesDisplay: "web" };
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
      transformCallback(callback: (value: unknown) => void) { const id = next++; callbacks.set(id, callback); return id; },
      unregisterCallback(id: number) { callbacks.delete(id); },
      async invoke(command: string, args: Record<string, any> = {}) {
        w.calls.push({ command, args });
        const account = { service: "moodle", baseUrl: "https://school.example/moodle/", username: "student", displayName: "Student" };
        const course = { id: 7, fullName: "Mathematik 10b", shortName: "M10b", enrolled: true, visible: true, sync: true, progress: null, endDate: null };
        if (command === "plugin:event|listen") {
          listeners.set(args.event, [...(listeners.get(args.event) ?? []), args.handler]);
          return next++;
        }
        if (command.startsWith("plugin:")) return null;
        if (command === "get_settings") return structuredClone(w.settings);
        if (command === "save_settings") { w.settings = structuredClone(args.settings); sessionStorage.setItem("mock.settings", JSON.stringify(w.settings)); return null; }
        if (command === "hybrid_password_available") return w.savedPasswordAvailable;
        if (command === "hybrid_saved_credentials") { if (!w.savedPasswordAvailable) throw { code: "browser", message: "No saved password" }; return { username: "fixture-user", password: "fixture-password" }; }
        if (command === "list_accounts") return [account, { ...account, service: "untis", baseUrl: "https://school.webuntis.com/" }];
        if (command === "message_counts" || command === "refresh_message_counts") return { chats: 0, mail: 0, notifications: 0, untis: 0 };
        if (command === "sync_status") return { running: false, lastAt: null, error: null, results: [] };
        if (command === "moodle_courses" || command === "moodle_refresh_courses") return [course];
        if (command === "moodle_file_stats") return {};
        if (command === "moodle_course" || command === "moodle_refresh_course") return { courseId: 7, sections: [], fetchedAt: 1 };
        if (command === "hybrid_open") {
          await new Promise((resolve) => setTimeout(resolve, 40));
          if (w.failBrowser) throw { code: "browser", message: "Test browser unavailable" };
          for (const id of listeners.get("hybrid-status") ?? []) callbacks.get(id)?.({ event: "hybrid-status", id, payload: { site: args.site, loading: false, error: null, downloaded: false } });
          return null;
        }
        if (command === "untis_homework" || command === "untis_exams") return { items: [], offline: false };
        if (command === "moodle_conversations") return [];
        if (command === "mail_status") return null;
        if (command === "untis_compose") return { canSend: false, recipients: [] };
        return [];
      },
    };
  });
  await page.goto("/");
});

test("course opens original page, resizes with sidebar, and retains local fallback", async ({ page }) => {
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  await expect(page.getByRole("radio", { name: "Native Anzeige", exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_open").length)).toBe(1);
  const open = await page.evaluate(() => (window as any).calls.find((c: any) => c.command === "hybrid_open"));
  expect(open.args.site).toBe("moodle");
  expect(open.args.target).toBe("https://school.example/moodle/course/view.php?id=7");
  expect(open.args.courseId).toBe(7);
  expect(open.args.bounds.x).toBeGreaterThan(200);
  expect(open.args.bounds.height).toBeGreaterThan(100);
  await page.getByRole("button", { name: "Seitenleiste einklappen" }).click();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_bounds").at(-1)?.args.bounds.x)).toBeLessThan(120);
  await page.getByRole("radio", { name: "Native Anzeige", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Web-Anzeige", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Kursordner öffnen" })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).calls.some((c: any) => c.command === "hybrid_hide"))).toBe(true);
});

test("Moodle messages default to native; Untis defaults to web and choices persist", async ({ page }) => {
  await page.getByRole("button", { name: "Mitteilungen", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Native Anzeige", exact: true })).toHaveAttribute("aria-checked", "true");
  expect(await page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_open").length)).toBe(0);
  await page.getByRole("tab", { name: "Benachrichtigungen", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Native Anzeige", exact: true })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("tab", { name: "Untis", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_open").at(-1)?.args.site)).toBe("untis");
  await expect(page.getByRole("radio", { name: "Web-Anzeige", exact: true })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("radio", { name: "Native Anzeige", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).settings.untisMessagesDisplay)).toBe("native");
  await page.reload();
  await page.getByRole("button", { name: "Mitteilungen", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Native Anzeige", exact: true })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("tab", { name: "Nachrichten", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Native Anzeige", exact: true })).toHaveAttribute("aria-checked", "true");
});

test("browser failures leave navigation and offline course view accessible", async ({ page }) => {
  await page.evaluate(() => { (window as any).failBrowser = true; });
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Test browser unavailable");
  await page.getByRole("radio", { name: "Native Anzeige", exact: true }).click();
  await expect(page.getByRole("radio", { name: "Web-Anzeige", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Einstellungen", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Einstellungen", exact: true })).toBeVisible();
});

test("saved passwords are requested only on click and hidden again automatically", async ({ page }) => {
  await page.clock.install();
  await page.evaluate(() => { (window as any).savedPasswordAvailable = true; });
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  const reveal = page.getByRole("button", { name: "Gespeicherte Zugangsdaten anzeigen", exact: true });
  await expect(reveal).toBeVisible();
  expect(await page.evaluate(() => (window as any).calls.some((c: any) => c.command === "hybrid_saved_credentials"))).toBe(false);
  await reveal.click();
  await expect(page.getByLabel("Gespeicherter Benutzername", { exact: true })).toHaveValue("fixture-user");
  await expect(page.getByLabel("Gespeichertes Passwort", { exact: true })).toHaveValue("fixture-password");
  expect(await page.evaluate(() => JSON.stringify(localStorage) + JSON.stringify(sessionStorage))).not.toContain("fixture-password");
  await page.clock.fastForward(30000);
  await expect(page.getByLabel("Gespeichertes Passwort", { exact: true })).toHaveCount(0);
});

test("without a saved password no reveal button appears; course display survives reload", async ({ page }) => {
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  await expect(page.getByRole("button", { name: "Gespeicherte Zugangsdaten anzeigen", exact: true })).toHaveCount(0);
  await page.getByRole("radio", { name: "Native Anzeige", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).settings.courseDisplay)).toBe("native");
  await page.reload();
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  await expect(page.getByRole("radio", { name: "Native Anzeige", exact: true })).toHaveAttribute("aria-checked", "true");
  expect(await page.evaluate(() => (window as any).calls.some((c: any) => c.command === "hybrid_open"))).toBe(false);
});

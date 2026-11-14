import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as unknown as Record<string, any>;
    const callbacks = new Map<number, (value: unknown) => void>();
    const listeners = new Map<string, number[]>();
    let next = 1;
    w.calls = [];
    w.failBrowser = false;
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
        if (command === "get_settings") return { locale: "de", theme: "light", subjectColors: {}, syncDir: "/tmp/sync", syncEnabled: true, syncIntervalMinutes: 30, notifications: true };
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
        return [];
      },
    };
  });
  await page.goto("/");
});

test("course opens original page, resizes with sidebar, and retains local fallback", async ({ page }) => {
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  await expect(page.getByRole("button", { name: "Lokale Ansicht", exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_open").length)).toBe(1);
  const open = await page.evaluate(() => (window as any).calls.find((c: any) => c.command === "hybrid_open"));
  expect(open.args.site).toBe("moodle");
  expect(open.args.target).toBe("https://school.example/moodle/course/view.php?id=7");
  expect(open.args.courseId).toBe(7);
  expect(open.args.bounds.x).toBeGreaterThan(200);
  expect(open.args.bounds.height).toBeGreaterThan(100);
  await page.getByRole("button", { name: "Seitenleiste einklappen" }).click();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_bounds").at(-1)?.args.bounds.x)).toBeLessThan(120);
  await page.getByRole("button", { name: "Lokale Ansicht", exact: true }).click();
  await expect(page.getByRole("button", { name: "Originalseite", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Kursordner öffnen" })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).calls.some((c: any) => c.command === "hybrid_hide"))).toBe(true);
});

test("messages route Moodle and Untis to separate browser views", async ({ page }) => {
  await page.getByRole("button", { name: "Mitteilungen", exact: true }).click();
  await expect(page.getByRole("button", { name: "Lokale Ansicht", exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_open").at(-1)?.args.target)).toBe("message/index.php");
  await page.getByRole("button", { name: "WebUntis", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).calls.filter((c: any) => c.command === "hybrid_open").at(-1)?.args.site)).toBe("untis");
  await page.getByRole("button", { name: "Neu laden", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).calls.some((c: any) => c.command === "hybrid_action" && c.args.action === "reload" && c.args.site === "untis"))).toBe(true);
  await page.getByRole("button", { name: "Einstellungen", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).calls.some((c: any) => c.command === "hybrid_hide" && c.args.site === "untis"))).toBe(true);
});

test("browser failures leave navigation and offline course view accessible", async ({ page }) => {
  await page.evaluate(() => { (window as any).failBrowser = true; });
  await page.getByRole("button", { name: "Kurse", exact: true }).click();
  await page.getByText("Mathematik 10b", { exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Test browser unavailable");
  await page.getByRole("button", { name: "Lokale Ansicht", exact: true }).click();
  await expect(page.getByRole("button", { name: "Originalseite", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Einstellungen", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Einstellungen", exact: true })).toBeVisible();
});

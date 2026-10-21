import { mount } from "svelte";
import App from "./App.svelte";
import { api } from "./lib/api";
import { i18n } from "./lib/i18n/index.svelte";
import "./app.css";

const target = document.getElementById("app");
if (!target) throw new Error("#app fehlt");

// Sprache vor dem ersten Rendern setzen, damit nichts umspringt.
const settings = await api.getSettings().catch(() => null);
i18n.use(settings?.locale ?? null);

export default mount(App, { target });

import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { resolveInitialLocale } from "./lib/i18n/initialLocale";
import { fetchUserSettings } from "./lib/settings";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Could not find #app mount element");
}

// Block the mount, bounded: resolve the persisted locale (or fall back to
// `navigator.languages` on timeout, rejection, or an absent/unsupported
// value) before the first paint, so a returning user with an explicit
// choice never sees a one-frame flash of their system language (design
// "Decision 1"). No top-level `await`, so no build-target dependency.
export default resolveInitialLocale(fetchUserSettings, navigator.languages).then((initialLocale) =>
  mount(App, { target, props: { initialLocale } }),
);

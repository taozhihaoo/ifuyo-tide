/**
 * 1.1.0 i18n — UI locale state ("en" default, "zh-CN").
 *
 * The locale persists in the settings KV (ui_locale) via the backend;
 * React re-renders through the subscribe/notify pair (wired into
 * useSyncExternalStore by lib/useStrings).
 */
import { call } from "./ipc";

export type Locale = "en" | "zh-CN";

let current: Locale = "en";
const listeners = new Set<() => void>();

export function getLocale(): Locale {
  return current;
}

export function subscribeLocale(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function notify(): void {
  for (const listener of listeners) listener();
}

export function setLocale(locale: Locale): void {
  if (locale === current) return;
  current = locale;
  notify();
  void call("set_ui_locale", { locale }).catch(() => {});
}

/** Reads the persisted locale once at startup (App mount). A failed
 * read keeps English — never a startup failure. */
export async function loadLocale(): Promise<void> {
  try {
    const stored = await call<string>("get_ui_locale");
    if (stored === "zh-CN" || stored === "en") {
      current = stored;
      notify();
    }
  } catch {
    // default stays "en"
  }
}

import { useSyncExternalStore } from "react";

/** Reactive locale for components that only need the locale itself. */
export function useLocale(): Locale {
  return useSyncExternalStore(subscribeLocale, getLocale);
}

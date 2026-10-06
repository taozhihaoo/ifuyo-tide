import { call } from "../../lib/ipc";

export function getUiLocale(): Promise<string> {
  return call<string>("get_ui_locale");
}

export function setUiLocale(locale: string): Promise<void> {
  return call<void>("set_ui_locale", { locale });
}

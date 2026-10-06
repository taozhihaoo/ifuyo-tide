import { useSyncExternalStore } from "react";
import { getLocale, subscribeLocale } from "./i18n";
import { stringsFor, type Strings } from "./strings";

/** The active string dictionary, reactive to locale changes. */
export function useStrings(): Strings {
  return useSyncExternalStore(subscribeLocale, () => stringsFor(getLocale()));
}

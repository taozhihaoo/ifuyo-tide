import { call } from "../../lib/ipc";

/** M9(上) §6: onboarding completion flag — persisted in the settings
 * KV; a fresh install (absent key) shows the welcome exactly once. */
export function getOnboardingCompleted(): Promise<boolean> {
  return call<boolean>("get_onboarding_completed");
}

export function setOnboardingCompleted(): Promise<void> {
  return call<void>("set_onboarding_completed");
}

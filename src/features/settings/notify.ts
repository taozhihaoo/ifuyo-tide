import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

/**
 * M8 §15: Windows notifications — REAL plugin calls; a notification
 * failure is logged and swallowed: it must never fail the download
 * (§39 "Notification failure ≠ Download failure"). No marketing or
 * recommendation notifications, ever (§15.2).
 */
export async function notify(title: string, body: string): Promise<void> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }
    if (granted) {
      sendNotification({ title, body });
    }
  } catch {
    // §72 degraded mode: notifications unavailable → downloads continue.
  }
}

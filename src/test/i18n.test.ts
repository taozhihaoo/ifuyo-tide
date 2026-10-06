import { describe, expect, it, vi } from "vitest";
import { getLocale, setLocale, subscribeLocale } from "../lib/i18n";
import { strings, stringsFor } from "../lib/strings";

/**
 * 1.1.0 i18n: the zh dictionary must mirror the en key tree exactly
 * (compile-time via `zh: typeof en`, runtime-sampled here), and the
 * locale switch must be observable.
 */
describe("i18n (1.1.0)", () => {
  it("defaults to English", () => {
    expect(getLocale()).toBe("en");
    expect(strings.downloads.emptyTitle).toBe("No downloads yet");
  });

  it("serves the zh dictionary on switch and keeps keys aligned", () => {
    const notify = vi.fn();
    const unsub = subscribeLocale(notify);
    setLocale("zh-CN");
    expect(getLocale()).toBe("zh-CN");
    expect(notify).toHaveBeenCalled();

    const zh = stringsFor("zh-CN");
    expect(zh.downloads.emptyTitle).toBe("还没有下载任务");
    expect(zh.downloadsView.technicalDetails).toBe("技术细节");
    expect(zh.torrentsView.filesSelected(5)).toBe("已选 5 个文件");
    expect(zh.settingsView.language).toBe("界面语言");

    // interpolation signatures survive the locale switch
    expect(typeof zh.downloads.startedNotice).toBe("function");
    expect(zh.downloads.startedNotice("abcdefgh1234")).toContain("abcdefgh");

    setLocale("en");
    unsub();
    expect(stringsFor("en").downloads.emptyTitle).toBe("No downloads yet");
  });
});

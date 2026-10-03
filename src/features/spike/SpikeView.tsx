import { useCallback, useEffect, useRef, useState } from "react";
import * as spike from "../../lib/spikeApi";
import type {
  MediaFixture,
  SpikeEvent,
  SpikeTorrentStatus,
} from "../../lib/spikeApi";

// M0.5 SPIKE — experiment driver UI. Deliberately utilitarian: this page
// exists to run and record experiments, not to look like a product.

const MATRIX_FILES = [
  "mp4_h264_aac.mp4",
  "webm_vp9_opus.webm",
  "webm_av1_opus.webm",
  "mkv_h264_aac.mkv",
  "mp4_hevc_aac.mp4",
];

interface MatrixRow {
  file: string;
  size: number;
  loadMs: number | null;
  durationSec: number | null;
  playOk: boolean;
  tFirstFrameMs: number | null;
  seekMs: number | null;
  error: string | null;
  state: "pending" | "running" | "done";
}

export function SpikeView() {
  const [media, setMedia] = useState<MediaFixture[]>([]);
  const [status, setStatus] = useState<spike.SpikeRuntimeInfo | null>(null);
  const [rows, setRows] = useState<MatrixRow[]>([]);
  const [torrent, setTorrent] = useState<SpikeTorrentStatus | null>(null);
  const [events, setEvents] = useState<SpikeEvent[]>([]);
  const [playUrl, setPlayUrl] = useState<string | null>(null);
  const [playSource, setPlaySource] = useState<string>("");
  const [note, setNote] = useState<string>("");
  const videoRef = useRef<HTMLVideoElement>(null);

  const refreshStatus = useCallback(async () => {
    setStatus(await spike.spikeStatus());
    setTorrent(await spike.spikeTorrentStatus());
  }, []);

  useEffect(() => {
    void (async () => {
      setMedia(await spike.spikeListMedia().catch(() => []));
      await refreshStatus();
    })();
  }, [refreshStatus]);

  const pollEvents = useCallback(async () => {
    setEvents((await spike.spikeEvents()).slice(-40).reverse());
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => void pollEvents(), 2000);
    return () => window.clearInterval(timer);
  }, [pollEvents]);

  useEffect(() => {
    if (!torrent || torrent.finished) return;
    const timer = window.setInterval(async () => {
      setTorrent(await spike.spikeTorrentStatus());
    }, 1500);
    return () => window.clearInterval(timer);
  }, [torrent]);

  // ---------- experiment A: format matrix ----------

  const runMatrix = useCallback(async () => {
    await spike.spikeStartFixtureServer();
    const port = (await spike.spikeStatus()).fixturePort;
    if (!port) return;

    for (const file of MATRIX_FILES) {
      const fixture = media.find((m) => m.name === file);
      setRows((r) => [
        ...r.filter((row) => row.file !== file),
        {
          file,
          size: fixture?.size ?? 0,
          loadMs: null,
          durationSec: null,
          playOk: false,
          tFirstFrameMs: null,
          seekMs: null,
          error: null,
          state: "running",
        },
      ]);

      const result = await runSingleFormat(port, file);
      await spike.spikeReportMediaResult({ ...result, userAgent: navigator.userAgent });
      setRows((r) => [
        ...r.filter((row) => row.file !== file),
        { ...result, size: fixture?.size ?? 0, state: "done" },
      ]);
    }
    setNote("Matrix complete — results stored to results-media.json");
  }, [media]);

  // ---------- experiment B: http playback ----------

  const playHttp = useCallback(async () => {
    const port = await spike.spikeStartFixtureServer();
    const session = await spike.spikeOpenPlaybackSession("http");
    setPlayUrl(session.url);
    setPlaySource(`HTTP fixture (${session.source})`);
    await refreshStatus();
    void port;
  }, [refreshStatus]);

  // ---------- experiments C-F/H/I/K: torrent chain ----------

  const startTorrent = useCallback(
    async (mode: "single" | "multi") => {
      setNote(`Starting ${mode} torrent (seed + download on loopback)…`);
      try {
        const status = await spike.spikeStartTorrent(mode);
        setTorrent(status);
        setNote(
          `${mode} torrent running: ${status.infoHash} (${status.fileProgress.length} files)`,
        );
      } catch (err) {
        setNote(`torrent start failed: ${String(err)}`);
      }
      await refreshStatus();
    },
    [refreshStatus],
  );

  // ---------- hands-free autorun (--spike-autorun) ----------

  const runTorrentChain = useCallback(async () => {
    const report: Record<string, unknown> = { kind: "autorun-torrent" };
    try {
      const started = Date.now();
      const st = await spike.spikeStartTorrent("single");
      setTorrent(st);
      report.setupMs = Date.now() - started;
      report.infoHash = st.infoHash;
      report.totalBytes = st.totalBytes;

      // wait until a meaningful part is present (>=5% or 45s)
      const t0 = Date.now();
      let status = st;
      while (Date.now() - t0 < 45_000) {
        status = (await spike.spikeTorrentStatus()) ?? st;
        if (status.progressBytes >= st.totalBytes * 0.05) break;
        await sleep(1000);
      }
      report.progressAtPlayStart = status.progressBytes;

      // experiment K sample: throttle the gateway during first playback
      await spike.spikeGatewaySlowMs(200);
      const session = await spike.spikeOpenPlaybackSession("torrent");
      setPlayUrl(session.url);
      setPlaySource(`Gateway → torrent (${session.source}) [autorun]`);

      const video = await waitForVideo(videoRef);
      video.muted = true;
      const tLoad = performance.now();
      await waitForEvent(video, "loadedmetadata", 60_000);
      report.durationSec = video.duration;
      await video.play();
      await waitForEvent(video, "playing", 120_000);
      report.tFirstFrameMs = Math.round(performance.now() - tLoad);
      report.videoWidth = video.videoWidth;
      report.videoHeight = video.videoHeight;
      report.playing = true;

      // H4: seek to 50% — target data is very likely not downloaded yet
      const seekStarted = performance.now();
      video.currentTime = video.duration * 0.5;
      await waitForEvent(video, "seeked", 180_000);
      report.seekTo50Ms = Math.round(performance.now() - seekStarted);
      await waitForEvent(video, "playing", 180_000).catch(() => {
        report.resumedAfterSeek = false;
      });
      report.resumedAfterSeek = report.resumedAfterSeek ?? true;
      await spike.spikeGatewaySlowMs(0);

      const finalStatus = await spike.spikeTorrentStatus();
      report.finalProgressBytes = finalStatus?.progressBytes;
      report.finished = finalStatus?.finished ?? false;
      report.wallMs = Date.now() - started;
      report.ok = true;
      setNote("AUTORUN: torrent chain complete — see results-media.json");
    } catch (err) {
      report.ok = false;
      report.error = String(err);
      setNote(`AUTORUN torrent chain failed: ${String(err)}`);
    } finally {
      await spike.spikeGatewaySlowMs(0);
    }
    await spike.spikeReportMediaResult(report);
  }, []);

  useEffect(() => {
    void (async () => {
      if (!(await spike.spikeAutorunEnabled())) return;
      setNote("AUTORUN: running full experiment sequence…");
      try {
        await runMatrix();
      } catch (err) {
        await spike.spikeReportMediaResult({
          kind: "autorun-matrix",
          ok: false,
          error: String(err),
        });
      }
      await runTorrentChain();
    })();
  }, [runMatrix, runTorrentChain]);

  const playTorrent = useCallback(async () => {
    const session = await spike.spikeOpenPlaybackSession("torrent");
    setPlayUrl(session.url);
    setPlaySource(`Gateway → torrent (${session.source})`);
  }, []);

  const playLocalFile = useCallback(async () => {
    const session = await spike.spikeOpenPlaybackSession("file");
    setPlayUrl(session.url);
    setPlaySource(`Gateway → file (${session.source})`);
  }, []);

  const seekTo = useCallback(async (fraction: number) => {
    const video = videoRef.current;
    if (!video || !Number.isFinite(video.duration)) return;
    const target = Math.min(video.duration * fraction, video.duration - 0.5);
    const started = performance.now();
    await new Promise<void>((resolve) => {
      const done = () => {
        video.removeEventListener("seeked", done);
        resolve();
      };
      video.addEventListener("seeked", done);
      video.currentTime = Math.max(target, 0);
    });
    await spike.spikeReportMediaResult({
      kind: "gateway-seek",
      targetSec: target,
      seekMs: Math.round(performance.now() - started),
      at: new Date().toISOString(),
    });
  }, []);

  return (
    <section className="view spike-view" aria-label="Spike">
      <div className="view-header">
        <h2>M0.5 Spike</h2>
        <span className="dim">
          experimental — results land in %LOCALAPPDATA%\ifuyo\Tide\spike\
        </span>
      </div>

      {note && <div className="banner banner-ok">{note}</div>}

      <div className="spike-status">
        <span>fixture: {status?.fixturePort ?? "stopped"}</span>
        <span>gateway: {status?.gatewayPort ?? "stopped"}</span>
        <span>torrent: {status?.torrentRunning ? "running" : "stopped"}</span>
        <button type="button" className="btn btn-small" onClick={() => void spike.spikeShutdown().then(refreshStatus)}>
          Shutdown spike runtime
        </button>
      </div>

      <h3>Experiment A — WebView2 format matrix (muted, automated)</h3>
      <button type="button" className="btn" disabled={media.length === 0} onClick={() => void runMatrix()}>
        Run 5-format matrix
      </button>
      <table className="spike-table">
        <thead>
          <tr>
            <th>file</th>
            <th>load ms</th>
            <th>duration</th>
            <th>play</th>
            <th>T first frame</th>
            <th>seek ms</th>
            <th>error</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.file}>
              <td>{row.file}</td>
              <td>{row.loadMs ?? "—"}</td>
              <td>{row.durationSec?.toFixed(1) ?? "—"}</td>
              <td>{row.state === "running" ? "…" : row.playOk ? "yes" : "no"}</td>
              <td>{row.tFirstFrameMs ?? "—"}</td>
              <td>{row.seekMs ?? "—"}</td>
              <td className="spike-error">{row.error ?? ""}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h3>Player — gateway / fixture playback (B, H)</h3>
      <div className="spike-row">
        <button type="button" className="btn" onClick={() => void playHttp()}>
          Play via HTTP fixture
        </button>
        <button type="button" className="btn" onClick={() => void playLocalFile()}>
          Play via gateway (file)
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => void playTorrent()}
          disabled={!torrent}
        >
          Play via gateway (torrent)
        </button>
      </div>
      {playUrl && (
        <div>
          <video
            ref={videoRef}
            src={playUrl}
            controls
            width={640}
            data-testid="spike-video"
            onWaiting={() => void spike.spikeReportMediaResult({ kind: "stall", at: new Date().toISOString() })}
          />
          <div className="dim">source: {playSource}</div>
          <div className="spike-row">
            <button type="button" className="btn btn-small" onClick={() => void seekTo(0.25)}>
              Seek 25%
            </button>
            <button type="button" className="btn btn-small" onClick={() => void seekTo(0.5)}>
              Seek 50%
            </button>
            <button type="button" className="btn btn-small" onClick={() => void seekTo(0.9)}>
              Seek 90%
            </button>
            <button
              type="button"
              className="btn btn-small"
              onClick={() => {
                const video = videoRef.current;
                if (video) video.currentTime += 30;
              }}
            >
              +30s
            </button>
          </div>
        </div>
      )}

      <h3>Experiments C-F/I/K — torrent harness</h3>
      <div className="spike-row">
        <button type="button" className="btn" onClick={() => void startTorrent("single")}>
          Start single torrent
        </button>
        <button type="button" className="btn" onClick={() => void startTorrent("multi")}>
          Start multi-file torrent
        </button>
        <button type="button" className="btn" disabled={!torrent} onClick={() => void spike.spikePauseTorrent().then(refreshStatus)}>
          Pause download
        </button>
        <button type="button" className="btn" disabled={!torrent} onClick={() => void spike.spikeResumeTorrent().then(refreshStatus)}>
          Resume download
        </button>
      </div>
      {torrent && (
        <div className="dim">
          {torrent.infoHash} · {(torrent.progressBytes / 1024 / 1024).toFixed(1)} MB /{" "}
          {(torrent.totalBytes / 1024 / 1024).toFixed(1)} MB ·{" "}
          {torrent.finished ? "finished" : torrent.paused ? "paused" : "downloading"}
          {torrent.error ? ` · error: ${torrent.error}` : ""}
        </div>
      )}
      <div className="spike-row">
        <label>
          gateway slow mode (ms/chunk):{" "}
          <select
            defaultValue="0"
            onChange={(e) => void spike.spikeGatewaySlowMs(Number(e.target.value))}
          >
            <option value="0">off</option>
            <option value="50">50</option>
            <option value="200">200</option>
            <option value="1000">1000</option>
          </select>
        </label>
        <label>
          read timeout (ms):{" "}
          <input
            type="number"
            defaultValue={120000}
            onBlur={(e) => void spike.spikeGatewayReadTimeoutMs(Number(e.target.value))}
          />
        </label>
      </div>

      <h3>Event log (last 40)</h3>
      <button type="button" className="btn btn-small" onClick={() => void pollEvents()}>
        Refresh
      </button>
      <table className="spike-table">
        <thead>
          <tr>
            <th>time</th>
            <th>kind</th>
            <th>detail</th>
          </tr>
        </thead>
        <tbody>
          {events.map((event, index) => (
            <tr key={`${event.tsUnixMs}-${index}`}>
              <td>{new Date(event.tsUnixMs).toLocaleTimeString()}</td>
              <td>{event.kind}</td>
              <td className="spike-detail">{JSON.stringify(event.detail)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

/** Runs one format through load → play → seek and measures timings. */
async function runSingleFormat(
  fixturePort: number,
  file: string,
): Promise<Omit<MatrixRow, "size" | "state">> {
  return new Promise((resolve) => {
    const video = document.createElement("video");
    video.muted = true; // muted playback is always allowed; documented in the report
    video.preload = "auto";
    const url = `http://127.0.0.1:${fixturePort}/media/${file}`;
    const started = performance.now();
    let tFirstFrame: number | null = null;
    let settled = false;

    const timeout = window.setTimeout(() => finish("timeout after 20s"), 20_000);

    function finish(error: string | null) {
      if (settled) return;
      settled = true;
      window.clearTimeout(timeout);
      video.removeAttribute("src");
      video.load();
      resolve({
        file,
        loadMs: Math.round(performance.now() - started),
        durationSec: Number.isFinite(video.duration) ? video.duration : null,
        playOk: tFirstFrame !== null,
        tFirstFrameMs: tFirstFrame,
        seekMs: null,
        error,
      });
    }

    video.addEventListener("loadedmetadata", () => {
      // try playback, then a mid-file seek
      video
        .play()
        .then(() => {
          video.addEventListener("playing", () => {
            if (tFirstFrame === null) {
              tFirstFrame = Math.round(performance.now() - started);
            }
            const target = Number.isFinite(video.duration)
              ? Math.min(video.duration / 2, video.duration - 0.5)
              : 1;
            const seekStarted = performance.now();
            video.addEventListener(
              "seeked",
              () => {
                // patch the seek latency into the result before resolving
                settled = true;
                window.clearTimeout(timeout);
                video.removeAttribute("src");
                video.load();
                resolve({
                  file,
                  loadMs: Math.round(performance.now() - started),
                  durationSec: Number.isFinite(video.duration) ? video.duration : null,
                  playOk: true,
                  tFirstFrameMs: tFirstFrame,
                  seekMs: Math.round(performance.now() - seekStarted),
                  error: null,
                });
              },
              { once: true },
            );
            video.currentTime = Math.max(target, 0);
          }, { once: true });
        })
        .catch((err: unknown) => finish(`play() rejected: ${String(err)}`));
    }, { once: true });

    video.addEventListener("error", () => {
      const code = video.error?.code;
      finish(`media error ${code ?? "?"} (${mediaErrorMessage(video.error?.code)})`);
    }, { once: true });

    video.src = url;
  });
}

function mediaErrorMessage(code?: number): string {
  switch (code) {
    case 1: return "aborted";
    case 2: return "network";
    case 3: return "decode";
    case 4: return "src-not-supported";
    default: return "unknown";
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

/** Resolves with the rendered <video> once React mounts it. */
async function waitForVideo(
  ref: React.RefObject<HTMLVideoElement | null>,
): Promise<HTMLVideoElement> {
  for (let i = 0; i < 100; i++) {
    if (ref.current) return ref.current;
    await sleep(100);
  }
  throw new Error("video element never mounted");
}

/** Waits for one DOM event with a hard timeout. */
function waitForEvent(
  target: HTMLVideoElement,
  event: string,
  timeoutMs: number,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      target.removeEventListener(event, onEvent);
      reject(new Error(`timeout waiting for "${event}" after ${timeoutMs}ms`));
    }, timeoutMs);
    function onEvent() {
      window.clearTimeout(timer);
      resolve();
    }
    target.addEventListener(event, onEvent, { once: true });
  });
}

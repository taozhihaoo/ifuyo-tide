# Tide Architecture (M0)

## Implemented today

```text
┌─────────────────────────────────────────────┐
│ Frontend — React + TypeScript + Vite        │
│ app/  components/  features/{tasks,library, │
│ player,settings}/  lib/{api,ipc,types}      │
└──────────────────┬──────────────────────────┘
                   │ Tauri IPC (typed commands)
                   │ structured {code,message} errors
┌──────────────────▼──────────────────────────┐
│ Application Core — Rust (src-tauri)         │
│                                             │
│ commands/   validate → domain → repo → DTO  │
│ security/   input validation at the boundary│
│ diagnostics/ structured JSON-lines logging  │
│                                             │
│ core/                                       │
│  ├─ task/      Task + TaskState machine     │
│  ├─ source/    Source + SourceType          │
│  ├─ resource/  ResourceId, ByteRange        │
│  ├─ scheduler/ Priority vocabulary (M7 stub)│
│  ├─ playback/  PlaybackSession              │
│  └─ library/   LibraryItem                  │
│                                             │
│ persistence/                                │
│  ├─ migration.rs  versioned, transactional  │
│  └─ repositories/ Task/Source/Settings      │
└──────────────────┬──────────────────────────┘
                   ▼
            SQLite (WAL, foreign keys)
     %LOCALAPPDATA%\ifuyo\Tide\database\tide.db
```

### Rules that hold the skeleton together

1. **Engine ≠ Scheduler.** Engines answer *how to get data*; the
   scheduler answers *what to get now*. Both come later; their seams
   (Resource, Availability, Demand, Priority) already exist as types.
2. **DownloadTask ≠ PlaybackSession.** Independent lifecycles by design.
3. **Frontend is semi-trusted.** All validation happens in Rust; the UI
   shows state, it never owns it.
4. **Domain → Repository → SQLite.** No SQL outside repositories, no
   domain objects serialized as JSON blobs.
5. **IDs are UUIDs**, never enumerable integers — this stays true for
   the future local stream gateway.

## Designed but NOT implemented in M0

```text
HTTP Engine        Torrent Engine (librqbit)
        │                 │
        └────────┬────────┘
                 ▼
             Resource          ← core/resource (types exist)
                 ▼
            Availability      ← M4
                 ▼
             Scheduler         ← M4 minimal / M7 full
                 ▼
      Progressive Stream Gateway ← M4 (loopback + capability token)
                 ▼
              Player           ← M5 (WebView2 first, mpv fallback)
```

## Milestone map

| M | Scope |
|---|---|
| M0 | Foundation (this document) |
| M0.5 | Technology spike — validates the chain above for real |
| M1 | HTTP download core |
| M2 | Persistence, crash recovery, resume |
| M3 | Torrent engine |
| M4 | Streaming / gateway / demand scheduler |
| M5 | Player |
| M6 | Library |
| M7 | Smart scheduler |
| M8 | Windows integration |
| M9 | Polish |
| M10 | Security & release hardening |
| M11 | Release candidate |
| M12 | 1.0 |

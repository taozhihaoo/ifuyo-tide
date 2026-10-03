# M2 — Recovery

How startup discovers interrupted tasks and decides what to do.

## Discovery (M2 §9/§84)

Recovery runs in `run()` BEFORE the frontend loads — never triggered by
the UI:

```text
open DB → migrate → open download repository
        → recover_all(repo, engine):
             for each persisted record:
                 filesystem facts (part size? final size? destination ok?)
                 → RecoveryDecision (pure, core/recovery/decision.rs)
                 → apply: persist corrected metadata
                 → register idle runtime entry with the engine
```

## Classification (M2 §10)

| Persisted state | Artifacts | Decision |
|---|---|---|
| any active | destination gone | BlockUntilUserAction (`destination_unavailable`) |
| completed | final exists | Keep (nothing to do) |
| completed | final MISSING | **MarkFailed** (`artifact_missing`) — Crash D: never keep a false Completed |
| cancelled / failed | any | Keep — settled; user must Retry/Resume explicitly (M2 §68) |
| active (created/ready/running/paused) | part == total > 0 | **Finalize** — Crash C: verify checksum → rename → Completed |
| active | part missing | MarkResumable from **0** (`artifact_missing`) |
| active | part present | MarkResumable from **part size** (`interrupted_runtime` / `artifact_larger_than_checkpoint` / `artifact_shorter_than_checkpoint`) |

## The resume point (M2 §59)

`safe_resume_bytes` = the PHYSICAL `.part` size — always the file, never
the DB number. Consequences:
- DB behind file (Crash A) → resume from file end; the un-checkpointed
  tail is real received data and is kept.
- DB ahead of file (the dangerous direction) → resume point moves BACK
  to the file size; the DB hint is corrected. The engine's sink then
  truncates the part to that offset before appending.

## User intent (M2 §68/§69)

- Paused → stays Paused. No auto-resume.
- Failed → stays Failed. No infinite auto-retry (Retry is explicit and
  keeps the task id, M2 §52).
- Cancelled → stays Cancelled, never silently resumes.
- Interrupted `running` → `ready` (resumable, idle) — the ONLY
  automatic reclassification, and it changes no user intent.

## Idempotency (M2 §32)

Every scan re-reads the RECONCILED truth: after the first pass the
records are `ready`/`completed`/`failed`/`cancelled` with DB bytes ==
part bytes, so a second scan produces `Keep`/same decisions — covered
by `crash_before_completion` (two scans) and the integration matrix.

## Validation on resume (M2 §36)

`resume`/`retry` re-run URL validation, destination planning and the
state-machine check on every call — the DB is not a security boundary.

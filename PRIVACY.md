# Privacy

## Principles

- **Local-first.** All data (tasks, settings, history) lives on your
  machine, in `%LOCALAPPDATA%\ifuyo\Tide\` and in the download directory
  you choose.
- **No account.** Tide has no login, no user profile, no cloud identity.
- **No analytics / no telemetry.** Verified again in M10 (§126: no
  analytics, crash reporting, update checker, or remote request was
  added by any milestone — see docs/M10-network-surface.md).
- **No cloud download service.** Tide talks directly to the sources you
  provide. No ifuyo server ever mediates, stores, or relays your traffic.

## What never leaves your machine

Download URLs, magnets, info hashes, file names, file paths, downloaded
content, playback history, download history, peer addresses, and usage
behaviour are never sent to ifuyo.

## What this is not

Tide is **not** an anonymity tool. We make no claims of hiding your IP
address, making you untrackable, or preventing connection records. When
Tide later uses BitTorrent, network connections to peers work exactly as
the protocol requires and are established directly from your machine.

## Logs

Production logs contain technical events only (task ids, state
transitions, error codes, timings). They never contain URLs, magnets,
file paths, peer IPs, or user-provided names. Detailed debugging data can
be enabled temporarily in development builds only.

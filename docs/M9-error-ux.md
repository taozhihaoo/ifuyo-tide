# M9 Error UX

The technical→domain→user mapping (M9上 §15/§16) and where each layer
lives (§17 collapsible details).

## The mapping chain

```text
Backend TideError { code, message }     (M1 §45 / M4(下) §98 taxonomy)
        ↓ lib/format describeDownloadError()
User explanation + recovery action      (banner copy)
        ↓ <details> collapsed
Technical code + raw message            (diagnostics only, §17)
```

## Download failure copy (§16/§85)

| Code | User sees | Recovery offered |
|---|---|---|
| DOWNLOAD_HTTP_STATUS | server returned an error | Retry / check link |
| DOWNLOAD_NETWORK | connection interrupted | Retry — resumes in place |
| DOWNLOAD_TIMEOUT | server stopped responding | Retry later |
| DOWNLOAD_CHECKSUM_MISMATCH | file damaged (checksum) | Retry (broken file kept) |
| DOWNLOAD_DISK_FULL | disk full | Free space / change destination |
| DOWNLOAD_FILE_WRITE / PERMISSION_DENIED | could not be written | Check folder permissions |
| DOWNLOAD_INVALID_URL | not a valid HTTP(S) URL | Check the link |
| DOWNLOAD_PRIVATE_ADDRESS_BLOCKED | private network address | Allow private if intended |
| (default) | The download failed | Retry |

## Player error copy

Media/decoder failures map to the M5 typed kinds (§32): Unsupported /
Decode / Network / ResourceUnavailable / Timeout / SessionExpired /
Gateway / ResourceClosed / Unknown — each with user-safe copy, a
recovery action when transient, and "Open in default app" as the
escape path for unsupported media (§33).

## Rules kept (§16/§55/§63/§70)

No blaming without evidence; no raw reqwest/rqbit text; short,
neutral, actionable; destructive actions confirm (§14); backend
validation intact behind every recovery action (§70).

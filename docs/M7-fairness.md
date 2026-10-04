# M7 Fairness & Aging

Within the leftover capacity (after Emergency minimums and Playback
targets), entries share by effective weight = base(class) × user
multiplier + aging bonus (§22). Same class + no overrides → equal
shares (§108 test). Aging (§21): +weight linear in waited-seconds,
capped at 0.5, applied ONLY to Normal/Background and only to entries
without a live session — a background task can rise toward mid-Normal
but can NEVER outrank Emergency/Playback (§110 test). Starvation is
guarded twice: proportional shares are nonzero while capacity exists,
and the §95 background floor reserves 20% of the leftover for
background once every playback buffer is ≥ target.

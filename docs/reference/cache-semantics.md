# Generic source cache: reference semantics

Reference implementation: `frog-pond/ccc-server` at commit
`54cf5862e4a09917e9bdc932fa1632fd1c3e0f49`. Every `file:line` below is relative
to that repository's root. This document describes what that code does, including
its edge cases. It does not say what the code should do. Where the code and
`workers/README.md` disagree, the code is taken as correct, and the disagreement
is listed in §16.

Terms used throughout:

- **object**: one `SourceDO` Durable Object instance. There is one per
  `(source name, key(params))`, addressed by the name `"<name>:<key>"`
  (`workers/src/client.ts:9`).
- **row**: the single `entry` row (`id = 1`) that the object stores
  (`workers/src/source-do.ts:46-58`).
- **load**: one run of `#load` (`workers/src/source-do.ts:188-237`), which is
  the only code that calls the source's `load` function.
- **now_r**: the clock value read at the start of a read
  (`workers/src/source-do.ts:90`). **now_l**: the clock value read at the start
  of a load (`workers/src/source-do.ts:191`). **now_s**: the clock value read
  when the next alarm is scheduled, after a load has finished
  (`workers/src/source-do.ts:244`). These are three separate reads of `clock.now()`
  (`workers/src/clock.ts:3`), and they can differ.

---

## 1. Constants

| Name | Value | Defined at | Governs |
|---|---|---|---|
| `MIN_BACKOFF` | 30 000 ms (30 s) | `workers/src/source-do.ts:15` | Backoff after the first consecutive failure; the base of the doubling (`workers/src/source-do.ts:223`). |
| `MAX_BACKOFF` | 3 600 000 ms (1 h) | `workers/src/source-do.ts:16` | Cap on the exponential part of the backoff. `Retry-After` can still exceed it (`workers/src/source-do.ts:222-225`). |
| `IDLE_AFTER` | 2 × 86 400 000 ms = 172 800 000 ms (2 days) | `workers/src/source-do.ts:18` | A source whose `last_read` is older than this (strict `>`) is no longer refreshed by its alarm (`workers/src/source-do.ts:245`, `:253`). |
| `TOUCH_EVERY` | 10 × 60 000 ms = 600 000 ms (10 min) | `workers/src/source-do.ts:20` | A read rewrites `last_read` only when the stored value is older than this (strict `>`) (`workers/src/source-do.ts:101`). |
| `MAX_KEPT` | 16 × 1024 × 1024 = 16 777 216 (string length, i.e. UTF-16 code units, not bytes) | `workers/src/conditional.ts:35` | A 200 body longer than this is not kept for conditional requests (`workers/src/conditional.ts:115`). |
| `MAX_KEPT_COMPRESSED` | 1024 × 1024 = 1 048 576 bytes (gzipped) | `workers/src/conditional.ts:37` | A kept body whose gzip is larger than this is not written (`workers/src/source-do.ts:168`). |
| `MAX_RETRY_AFTER` | 24 × 60 × 60 × 1000 ms = 86 400 000 ms (24 h) | `workers/src/conditional.ts:53` | Upper clamp on a parsed `Retry-After` (`workers/src/conditional.ts:82`). |
| `USER_AGENT` | `ccc-server/2.0` | `workers/src/upstream.ts:5` | `User-Agent` sent on every upstream request (`workers/src/upstream.ts:13`). |
| `CLIENT_MAX_AGE` | 600 s (10 min) | `workers/src/lifetimes.ts:3` | `Cache-Control: max-age` of most successful responses (§11). |
| `ERROR_MAX_AGE` | 60 s | `workers/src/lifetimes.ts:6` | `max-age` of 502 errors, food stand-ins, and WordPress `stale-error` answers (§11). Aliased as `ONE_MINUTE` at `workers/src/router.ts:53`. |
| `SOURCE_TTL` | 3 600 000 ms (1 h) | `workers/src/lifetimes.ts:10` | The `ttl` that every generic source in the repository declares (e.g. `workers/src/sources/bonapp.ts:77`, `workers/src/sources/athletics.ts:95`). |
| `DAY` (per source module) | 86 400 000 ms | e.g. `workers/src/sources/bonapp.ts:12`, `workers/src/sources/athletics.ts:18` | The `staleIfError` that every generic source declares (e.g. `workers/src/sources/bonapp.ts:78`). |
| ETag digest length | first 18 bytes of SHA-256, base64url → 24 characters, quoted | `workers/src/etag.ts:7-9` | Client-facing ETag (§11). |
| Athletics `MINUTE` / `BEFORE_KICKOFF` | 60 000 ms / 300 000 ms | `workers/src/sources/athletics.ts:17`, `:21` | Bounds of `athleticsFreshFor`, the only `ttlFor` in the repository (§8). |
| WordPress `MAX_STORED` | 1 900 000 bytes | `workers/src/sources/wordpress-api.ts:20` | Source-specific: larger answers make the load throw (`workers/src/sources/wordpress-api.ts:97-99`). Not part of the generic cache. |

None of the generic constants can be configured per source. Each source sets
only `ttl`, `staleIfError`, `epoch` and `ttlFor` (§2).

---

## 2. What a source declares

Interface: `workers/src/define-source.ts:1-18`. `defineSource` is the identity
function (`workers/src/define-source.ts:20`).

| Field | Type | Required | Meaning / use |
|---|---|---|---|
| `name` | `string` | yes | Namespace for the object name `"<name>:<key>"` (`workers/src/client.ts:9`) and the registry key (`workers/src/registry.ts:7`). The first read stores it in the row (`workers/src/source-do.ts:95-96`), and every load and schedule looks the spec up again by that stored name (`workers/src/source-do.ts:190`, `:243`). |
| `key` | `(params) => string` | yes | Identity of one upstream resource. It is used only by the caller, to name the object (`workers/src/client.ts:9`). The object never calls it. |
| `load` | `(params, env) => Promise<V>` | yes | Fetches and validates the upstream data. Throwing means failure (`workers/src/define-source.ts:6`). It is called with the params stored in the row, which are the first reader's params (`workers/src/source-do.ts:199-200`). It runs inside the conditional-request context (§10). |
| `ttl` | `number` (ms) | yes | Freshness duration when the row's `ttl` column is null (`workers/src/source-do.ts:110`, `:247`). |
| `ttlFor` | `(value, fetchedAt) => number` (ms) | optional | Freshness that depends on the loaded value. It is evaluated once per successful load and stored in the `ttl` column (`workers/src/source-do.ts:208`). §8. |
| `staleIfError` | `number` (ms) | yes | Length of the window after `ttl` in which a read answers the stored value at once and the alarm refreshes it (`workers/src/source-do.ts:115`). It does **not** limit how long a stored value is served when upstream fails (§6). |
| `epoch` | `(now: Date) => string` | optional | Freshness label. A stored value counts as fresh or stale only while its stored epoch equals `epoch(now_r)` (`workers/src/source-do.ts:108`). §7. |
| `record` | `(params, value, env, now) => Promise<void>` | optional | Called after a successful load and not awaited (`ctx.waitUntil`). A rejection is logged and ignored (`workers/src/source-do.ts:211-217`). It writes to `ArchiveDO`, which is out of scope. |

Registration: `registerSource` writes `registry[source.name]`, and a later
registration under the same name replaces the earlier one
(`workers/src/registry.ts:4-8`). The object receives only a name and params, not
functions (`workers/src/registry.ts:3`). A source has to be imported by the
worker module graph to be registered (README `workers/README.md:361-362`).

The stored value goes through `JSON.stringify` on write
(`workers/src/source-do.ts:205`) and `JSON.parse` on read
(`workers/src/source-do.ts:84`). A reader therefore receives the JSON round-trip
of what `load` returned, not the same object.

---

## 3. Stored state

### 3.1 `entry` table: one row per object, `id = 1`

Schema: `workers/src/source-do.ts:45-58`. The `ttl` column is added by
migration if missing (`workers/src/source-do.ts:59-65`). TypeScript row type:
`workers/src/source-do.ts:22-34`.

| Column | Type | Written when |
|---|---|---|
| `id` | INTEGER, always 1 | On insert (`workers/src/source-do.ts:95`). |
| `name` | TEXT NOT NULL | On insert only, from the first reader (`workers/src/source-do.ts:95-96`). |
| `params` | TEXT NOT NULL (JSON) | On insert only: `JSON.stringify(params)` of the first reader (`workers/src/source-do.ts:97`). Later readers' params are ignored. |
| `value` | TEXT, nullable (JSON) | On successful load: `JSON.stringify(value)` (`workers/src/source-do.ts:203-205`). |
| `fetched_at` | INTEGER ms, nullable | On successful load: **now_l**, the time the load started, not when it finished (`workers/src/source-do.ts:191`, `:206`). Null means no value has ever been stored (`workers/src/source-do.ts:106`). |
| `epoch` | TEXT, nullable | On successful load: `spec.epoch(new Date(now_l))`, or null when the spec has no `epoch` (`workers/src/source-do.ts:207`). |
| `ttl` | INTEGER ms, nullable | On successful load: `spec.ttlFor(value, now_l)`, or null when the spec has no `ttlFor` (`workers/src/source-do.ts:208`). |
| `failures` | INTEGER NOT NULL DEFAULT 0 | Set to 0 on successful load (`workers/src/source-do.ts:204`). Set to `failures + 1` on failed load (`workers/src/source-do.ts:220`, `:227-228`). The counter has no upper bound. |
| `backoff_until` | INTEGER ms NOT NULL DEFAULT 0 | Set to 0 on successful load (`workers/src/source-do.ts:204`). Set to `now_l + wait` on failed load (`workers/src/source-do.ts:229`). |
| `last_error` | TEXT, nullable | Set to NULL on successful load (`workers/src/source-do.ts:204`). Set to `String(err)` on failed load, e.g. `"Error: boom"` (`workers/src/source-do.ts:230`). |
| `last_read` | INTEGER ms NOT NULL | On insert: now_r (`workers/src/source-do.ts:98`). On a read when `now_r - last_read > TOUCH_EVERY` (`workers/src/source-do.ts:101-103`). Loads and alarms never write it. |

All columns of the row are written on insert. The value columns start NULL and
the counters start at their defaults (`failures = 0`, `backoff_until = 0`).

### 3.2 `response` table: kept upstream answers for conditional requests

Schema: `workers/src/source-do.ts:68-76`.

| Column | Type | Written when |
|---|---|---|
| `key` | TEXT PK | SHA-256 hex digest of the request URL (`workers/src/conditional.ts:56-59`). |
| `etag` | TEXT nullable | The `ETag` response header. |
| `last_modified` | TEXT nullable | The `Last-Modified` response header. |
| `headers` | TEXT NOT NULL | JSON array of `[name, value]` pairs. Names are lowercase as produced by Headers iteration, and `set-cookie` is removed (`workers/src/conditional.ts:116`, `workers/src/source-do.ts:174`). |
| `body` | BLOB NOT NULL | The gzipped body text (`workers/src/source-do.ts:163`, `workers/src/conditional.ts:40-43`). |

Write rule: only after a successful load, the whole table is deleted and then
refilled with the answers this load used. An answer is skipped when its gzip is
larger than `MAX_KEPT_COMPRESSED` (`workers/src/source-do.ts:159-178`, `:201`).
This step runs before the `entry` update in the same `try`. A load that fails
inside `spec.load` leaves the table unchanged, because the step is never reached
(`workers/src/source-do.ts:200-201`). A failure after the step has started, in
the step itself or in the `entry` update that follows it, is caught by the same
`catch` (`workers/src/source-do.ts:219-233`), which records the failure and does
not restore the table. The stored value can then be older than the kept answers.

### 3.3 Alarm

Each object has one Durable Object alarm. It is written at
`workers/src/source-do.ts:116` (`setAlarm(now_r)`), at `:248`
(`setAlarm(next)`), and deleted at `:245` and `:271`. §5.

### 3.4 In-memory state (not persisted)

`#inflight: Promise<Row> | null` (`workers/src/source-do.ts:41`) is the
single-flight handle (§4.3). Eviction loses it.

---

## 4. Read path (`get`)

Code: `workers/src/source-do.ts:87-131`. The caller is `fetchSource`
(`workers/src/client.ts:7-15`).

### 4.1 Steps before the decision (run in one synchronous turn)

1. `spec = registry[name]`. If it is missing, return
   `{state: 'error', error: 'unknown source: <name>'}` right away, before the
   clock is read or storage is touched (`workers/src/source-do.ts:88-89`).
2. `now_r = clock.now()` (`workers/src/source-do.ts:90`).
3. If no row exists, insert `(1, name, JSON(params), last_read = now_r)` and read
   it back (`workers/src/source-do.ts:92-100`). Otherwise, if
   `now_r - last_read > 600 000`, write `last_read = now_r`
   (`workers/src/source-do.ts:101-104`).
4. Derived inputs (`workers/src/source-do.ts:106-110`):
   - `V` (hasValue) = `fetched_at !== null`
   - `age` = `now_r - fetched_at`, or `Infinity` when there is no value
   - `E` (sameEpoch) = `spec.epoch` is absent **or** `row.epoch === spec.epoch(new Date(now_r))`
   - `B` (backedOff) = `backoff_until > now_r` (strict)
   - `ttl` = `row.ttl ?? spec.ttl` (null and undefined fall back; 0 is kept)
   - `T` (within ttl) = `age < ttl` (strict)
   - `W` (within the stale window) = `age < ttl + spec.staleIfError` (strict)
   - `F` (fetch in flight) = `#inflight !== null`. `get` never tests this
     directly. It matters only because `#refresh` joins an existing promise
     (`workers/src/source-do.ts:182-186`).

Every step up to the first `await` is synchronous. The first `await` is the
`setAlarm` at `:116` or the `#refresh` at `:124`. The row read, the decision and
the alarm write or load start therefore happen in one turn, and no other event
in the object can interleave between them.

### 4.2 Decision table

"Load" means `#refresh()`: a new `#load` starts if `#inflight` is null,
otherwise the reader joins the existing one. "Answer" is the `state` returned.
`f` = the row's `failures`.

| Row | Registered | V | E | T | W | B | F | Answer | Upstream fetch starts? | Reader waits? | Writes | Code |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| R0 | no | – | – | – | – | – | – | `error` "unknown source: name" | no | no | none (no row inserted, `last_read` not touched) | `source-do.ts:88-89` |
| R1 | yes | yes | yes | yes | (yes) | any | any | `fresh` (stored value) | no | no | only the §4.1 step 3 insert/touch | `source-do.ts:112` |
| R2 | yes | yes | yes | no | yes | no | any | `stale` if `f = 0`, `stale-error` if `f > 0` | no, not from the read. The alarm is set to now_r, and when it fires it loads (§5). | no (awaits only the `setAlarm`) | `setAlarm(now_r)`, replacing any pending alarm | `source-do.ts:115-118` |
| R3 | yes | yes | yes | no | yes | yes | any | `stale-error` (B implies `f > 0`, §4.4) | no | no | none beyond §4.1 step 3 (the pending alarm, normally at `backoff_until`, is kept) | `source-do.ts:115-118` |
| R4 | yes | yes | yes | no | no | no | no | success: `fresh` with the new value; failure: `stale-error` with the stored value | yes | yes | load writes (§6, §4.5) | `source-do.ts:122-129` |
| R5 | yes | yes | yes | no | no | no | yes | as R4, with the outcome of the joined load | no (joins) | yes | those of the joined load | `source-do.ts:124`, `:182-186` |
| R6 | yes | yes | yes | no | no | yes | any | `stale-error` (stored value) | no | no | none beyond §4.1 step 3 | `source-do.ts:122`, `:129` |
| R7 | yes | yes | no | any | any | no | no | success: `fresh` (new value); failure: `stale-error` (stored, earlier-epoch value). Age is not considered. | yes | yes | load writes | `source-do.ts:108`, `:112`, `:115`, `:122-129` |
| R8 | yes | yes | no | any | any | no | yes | as R7, with the outcome of the joined load | no (joins) | yes | those of the joined load | as R7 |
| R9 | yes | yes | no | any | any | yes | any | `stale-error` (stored, earlier-epoch value) | no | no | none beyond §4.1 step 3 | `source-do.ts:122`, `:129` |
| R10 | yes | no | n/a | n/a | n/a | no | no | success: `fresh`; failure: `error` with `last_error` (or `"upstream unavailable"` if null) | yes | yes | load writes | `source-do.ts:122-130` |
| R11 | yes | no | n/a | n/a | n/a | no | yes | as R10, with the outcome of the joined load | no (joins) | yes | those of the joined load | as R10 |
| R12 | yes | no | n/a | n/a | n/a | yes | any | `error` with `last_error ?? "upstream unavailable"` | no | no | none beyond §4.1 step 3 | `source-do.ts:130` |

Notes on the table:

- **Served value and `fetchedAt`.** `fresh` after a load returns the row that
  `#load` read back after its update (`workers/src/source-do.ts:218`, `:124`).
  Its `fetchedAt` is that load's now_l, which can be earlier than the reader's
  now_r, especially for a reader that joined. `stale` and `stale-error` return
  the stored `value` and `fetched_at` (`workers/src/source-do.ts:83-85`).
- **Failure re-read.** On failure, the reader re-reads the row
  (`workers/src/source-do.ts:126`). R4, R7 and R10 then choose between
  `stale-error` and `error` by checking `fetched_at !== null` on that re-read row
  (`workers/src/source-do.ts:129`).
- **Joined load failure.** Every reader joined to a failed load gets the same
  rejection. Each catches it and re-reads the row
  (`workers/src/source-do.ts:125-127`), and each answers the same way.
- **Reader arriving while a fetch is out.** Only R4/R5, R7/R8 and R10/R11 call
  `#refresh`, so only those readers share the fetch. A reader in R1, R2, R3, R6,
  R9 or R12 answers at once and does not wait for an in-flight load. This
  includes an R2 reader whose row has an alarm-started load running.
- **Fresh ignores failures.** R1 returns `fresh` without checking `failures` or
  `B` (`workers/src/source-do.ts:112`).
- **Error string.** For R10 failure and R12, the `error` string is the stored
  `String(err)`, e.g. `"Error: boom"`. `fetchSource` throws `new Error(<that
  string>)` (`workers/src/client.ts:12`).

### 4.3 Single-flight

`#refresh` assigns `#inflight ??= #load().finally(() => #inflight = null)`
(`workers/src/source-do.ts:182-186`). Readers in R4/R5, R7/R8 and R10/R11, and
the alarm handler (`workers/src/source-do.ts:256`), all get the same promise
while it is pending. The test `concurrent cold misses share one upstream fetch`
holds the load open and confirms that five readers cause one `load` call and
each gets `fresh` (`workers/test/source-do.test.ts:66-80`). `#inflight` is
cleared in a `.finally` that is chained after `#load` settles. `#load`'s own
`finally` runs `#schedule` first (`workers/src/source-do.ts:234-236`).

### 4.4 Combinations that cannot occur, or are not distinguished

- **`B` with `failures = 0`: cannot occur.** `backoff_until` becomes non-zero
  only in the failure branch, together with `failures ≥ 1`
  (`workers/src/source-do.ts:226-231`). It returns to 0 only together with
  `failures = 0` (`workers/src/source-do.ts:204`), or when the row is deleted
  (`workers/src/source-do.ts:269`). `wait ≥ 30 000 > 0`, so a failure always
  sets `backoff_until > now_l`. So R3 always answers `stale-error`, never
  `stale`. The reverse does not hold: `failures > 0` with `B` false is normal
  once the backoff has run out, and gives R2 `stale-error`.
- **`T` true with `W` false: cannot occur** when `staleIfError ≥ 0`, because
  `age < ttl` implies `age < ttl + staleIfError`.
- **`V` false: T, W and E do not apply.** `age = Infinity`, so T and W are
  false, and the `V` guard on lines 112 and 115 skips them. E is computed but
  not used.
- **`E` false: T and W do not matter.** Lines 112 and 115 both need `sameEpoch`.
  An earlier-epoch value is never `fresh` and never `stale` (from the stale
  window), whatever its age.
- **`staleIfError = 0`:** W equals T, so R2 and R3 cannot occur. Every
  non-fresh read loads in the foreground. `workers/test/conditional.test.ts:26-28`
  relies on this.
- **First-ever read:** the row has just been inserted with `backoff_until = 0`
  and `fetched_at = NULL`, so it is R10, or R11 if a load is somehow already in
  flight. No load can be in flight before the row exists, except after a purge
  that ran during a load (§9).
- **`F` with `B`:** rare. A failed load writes `backoff_until`
  (`workers/src/source-do.ts:226-231`) and then awaits `#schedule()`
  (`workers/src/source-do.ts:235`) before `#inflight` is cleared. A reader that
  arrives in that gap sees both B and F. It does not join, because no B row
  calls `#refresh`. An alarm that fires before `backoff_until` would also cause
  this, because `alarm()` does not check B (§5). The scheduler never sets such an
  alarm, but the test helper `runDurableObjectAlarm` runs the alarm on demand.
- **`F` with `T` (a load running while the value is fresh):** possible only
  through the alarm (§5: `alarm()` does not check freshness), or in the gap
  between a successful load's entry update and `#inflight` being cleared. The
  read answers R1 and does not join.
- **`error` with `"upstream unavailable"`:** reached only when the row has
  `fetched_at = NULL` and `last_error = NULL` after a failed or skipped load.
  Every recorded failure writes a non-null `String(err)`. This branch is
  therefore reached only when `#load` throws before its `try` at
  `workers/src/source-do.ts:198` (for example inside `#keptResponses`,
  `workers/src/source-do.ts:193`), when nothing is recorded. UNVERIFIED whether
  any real input can make `#keptResponses` throw. Undecodable bodies are skipped
  (`workers/src/source-do.ts:146-148`), but `JSON.parse(row.headers)` at `:152`
  is not guarded.

### 4.5 Ordering and concurrency of alarm writes from reads

R2's `setAlarm(now_r)` is issued in the same synchronous turn as its row read.
A load writes its entry update and calls `#schedule`'s `setAlarm`/`deleteAlarm`
in one synchronous turn as well. On success the order is update at `:202`, then
`waitUntil`, return, `finally`, `#schedule` (`:241-248`), with no `await` before
`setAlarm` is called. On failure it is update at `:226`, note, throw, `finally`,
`#schedule`. This gives two cases for an R2 reader and a running load:

- The reader's `setAlarm(now_r)` comes before the load completes. The load's
  `#schedule` then replaces it.
- The reader comes after the load completes. It then sees the updated row: R1
  after a success, or R3 after a failure (no alarm write). The exception is a
  failed load that took longer than its `wait`, so that `backoff_until ≤ now_r`.
  The reader is then R2 and sets the alarm to now_r.

The rule throughout is that the last `setAlarm` or `deleteAlarm` call wins.
UNVERIFIED: how the Durable Objects runtime treats a `setAlarm` made while the
alarm handler is running, and whether that run's completion clears a newly set
alarm. This needs Cloudflare's alarm documentation or a workerd test.

---

## 5. Background refresh (alarm)

### 5.1 When an alarm is set

- **Stale read (R2):** `setAlarm(now_r)`, i.e. due at once
  (`workers/src/source-do.ts:116`). It is set only when not backed off. Every R2
  read sets it again, with no deduplication.
- **After every load** that reaches its `try`, success or failure:
  `#schedule()` runs in `finally` (`workers/src/source-do.ts:234-236`,
  `:241-249`):
  - It reads the row, `spec = registry[row.name]`, and **now_s**.
  - If `now_s - last_read > IDLE_AFTER`, it calls `deleteAlarm()` and stops
    (`:245`).
  - Otherwise `next = backoff_until > now_s ? backoff_until : (fetched_at ?? now_s) + (row.ttl ?? spec.ttl)`,
    followed by `setAlarm(next)` (`:246-248`).
  - After a success, `backoff_until = 0`, so `next = fetched_at + ttl`, which is
    now_l + effective ttl. The test `the alarm refreshes ahead of demand and
    schedules the next one` checks that the alarm is `now + HOUR`
    (`workers/test/source-do.test.ts:100-110`).
  - After a failure, `next = backoff_until` when it is still in the future.
    If the load itself took longer than `wait`, so that
    `backoff_until ≤ now_s`, then `next = fetched_at + ttl`. That time can be in
    the past when a value is stored. With no stored value it is `now_s + ttl`.
    UNVERIFIED: that the runtime fires a past-dated alarm at once. Cloudflare
    documents this, but nothing in this repository tests it.
- **Not set** by a cold insert, by R0, R1, R3, R6, R9 or R12, or when a load
  fails before its `try` (§4.4).
- **Deleted** by `purge()` (`workers/src/source-do.ts:271`) and by `#schedule`
  when idle (`:245`).

### 5.2 What the alarm does when it fires

`alarm()` (`workers/src/source-do.ts:251-260`):

1. It reads the row. If there is no row, or `clock.now() - last_read > IDLE_AFTER`,
   it returns and does nothing: no load and no new alarm (`:252-253`). The alarm
   that fired is used up, so none is left. The test `a source nobody reads goes
   quiet` checks that `getAlarm()` is then null
   (`workers/test/source-do.test.ts:206-213`).
2. Otherwise it notes the trace attribute `ccc.source = row.name` (`:254`) and
   calls `#refresh()` (`:256`). That starts a load or joins the one in flight.
3. A rejection is caught and ignored (`:257-259`). The failure was already
   recorded and rescheduled in `#load`, and the comment says throwing would only
   make the runtime retry. The handler therefore always completes normally.

The alarm does **not** check freshness, epoch, or backoff. Whenever it fires on
a non-idle row, it loads. The tests use `runDurableObjectAlarm` to run it on
demand.

The alarm is scheduled from `fetched_at + ttl` only. It is **not** scheduled at
epoch boundaries, so a value whose epoch has expired is noticed only by the next
read, which then loads in the foreground (R7).

A failing upstream keeps being retried by the alarm alone, at each
`backoff_until`, until `last_read` is more than 2 days old. Reads are not needed
for those retries.

### 5.3 Eviction and cold start

- `entry`, `response` and the alarm live in Durable Object storage and survive
  eviction. The constructor runs `CREATE TABLE IF NOT EXISTS` for both tables
  and adds `ttl` if it is missing (`workers/src/source-do.ts:43-77`).
- `#inflight` is in memory only and is lost (`workers/src/source-do.ts:41`).
- After eviction, a read follows the same table using the stored state. The
  test `stored values survive eviction` evicts the object and then reads `fresh`
  without a new load (`workers/test/source-do.test.ts:112-119`).
- After a source has gone quiet (no alarm), a read resumes it only through R2
  (`setAlarm(now_r)`) or through a foreground load's `#schedule`. A read that
  answers R1, R3, R6 or R9 does not create an alarm.

---

## 6. Failure and backoff

### 6.1 What a failure is

A failure is any exception thrown inside the `try` of `#load`
(`workers/src/source-do.ts:198-219`). This covers `JSON.parse(params)`, the
source's `load` (including its validation), `#keepResponses` (gzip), the entry
`UPDATE`, and a missing spec (`registry[row.name]` undefined makes `spec.load`
throw a TypeError). `record` failures do **not** count, because `record` runs
under `waitUntil` with a `.catch` (`workers/src/source-do.ts:211-217`).

### 6.2 What is stored on failure

`failures = failures + 1`, `backoff_until = now_l + wait`, and
`last_error = String(err)` (`workers/src/source-do.ts:220-231`). `value`,
`fetched_at`, `epoch` and `ttl` are left unchanged. The `response` table is
unchanged too when the failure was inside `spec.load`; a failure after that
point can leave it rewritten (see the write rule in §3).
Trace attributes are noted (§13), the error is re-thrown (`:233`), and
`#schedule` runs (`:235`).

### 6.3 Exact sequence

`wait = max( min(30 000 × 2^(failures − 1), 3 600 000), retryAfter )`
(`workers/src/source-do.ts:222-225`). Here `failures` is the count after
incrementing, and `retryAfter` is the largest `Retry-After` noted during this
load (§10.4), 0 if none.

| Consecutive failure # | exponential part |
|---|---|
| 1 | 30 s |
| 2 | 60 s |
| 3 | 120 s |
| 4 | 240 s |
| 5 | 480 s |
| 6 | 960 s |
| 7 | 1 920 s |
| 8 and above | 3 600 s (capped; 30 s × 2^7 = 3 840 s > cap) |

The shortest hold is therefore the exponential step, at least 30 s. The longest
is `max(1 h, retryAfter)`, which is at most 24 h. The hold is measured from
**now_l**, the start of the failed load (`:229`), not from when it failed.

Tests: `backoff doubles with each consecutive failure`
(`workers/test/source-do.test.ts:169-183`) has failures at T (30 s) and T+31 s
(60 s, until T+91 s). At T+76 s it is still backed off, and at T+92 s it loads.
`a 503 without one backs off as any failure does` checks
`backoff_until = now + 30 000` (`workers/test/conditional.test.ts:137-141`).
`a 429 is not asked again before the wait it names` checks
`backoff_until = now + 2 h` from `Retry-After: 7200`
(`workers/test/conditional.test.ts:128-135`).

### 6.4 What backoff suppresses

While `B` holds, a read never starts a load (R3, R6, R9, R12) and R2 does not
set the alarm. The alarm handler does not check `B` itself (§5.2). It is
scheduled at `backoff_until`, so it loads when the backoff ends. The test
`backoff suppresses upstream calls, then lets one through`
(`workers/test/source-do.test.ts:152-167`) shows this. A failure past the stale
window sets a 30 s backoff. At +10 s the read is R6, `stale-error` with no load.
At +31 s the read is R4, `fresh` v2 with a load.

### 6.5 What resets it

Only a successful load. It sets `failures = 0`, `backoff_until = 0` and
`last_error = NULL` (`workers/src/source-do.ts:202-204`). `purge()` also removes
the row entirely (`workers/src/source-do.ts:269`). Backoff running out does not
reset `failures`. The next failure keeps doubling from where the count stands.

### 6.6 How `staleIfError` bounds (and does not bound) serving old values

`staleIfError` decides only **whether the read waits for a load**. It never
decides **whether a stored value may be served**.

- **Within it** (`ttl ≤ age < ttl + staleIfError`, same epoch): the read answers
  at once with the stored value. The state is `stale` if `failures = 0` and
  `stale-error` if `failures > 0`. The refresh happens on the alarm (R2), or is
  postponed under backoff (R3). The reader never waits and never starts a load.
  Test `within staleIfError, a value whose refresh failed is a stand-in`
  (`workers/test/source-do.test.ts:131-140`): `stale`, then the alarm fails,
  then `stale-error`.
- **Past it** (`age ≥ ttl + staleIfError`, same epoch): the read loads in the
  foreground and waits (R4/R5). Success gives `fresh`. Failure gives
  `stale-error` **with the stored value, however old it is**. When backed off
  (R6) the answer is `stale-error` at once. Test `past staleIfError, a failed
  refresh still serves the stored value` (`workers/test/source-do.test.ts:142-150`).
- No code path deletes or refuses a stored value because of its age. Only
  `purge()` removes it (§9).

---

## 7. Epochs

- **Computed:** at read time as `spec.epoch(new Date(now_r))`
  (`workers/src/source-do.ts:108`). At load time as `spec.epoch(new Date(now_l))`,
  which is stored in `epoch` (`workers/src/source-do.ts:207`).
- **Compared:** with strict string equality `row.epoch === current`. A spec
  without `epoch` always counts as same-epoch, even if the row holds a non-null
  epoch from earlier code. A spec with `epoch` and a row with `epoch = NULL`
  (stored by code that had no epoch) never matches.
- **Order:** epochs are never compared for order. "Earlier" only means
  "different". A value whose stored epoch differs from the current one, in
  either direction, is handled the same way.
- **Upstream up:** a different epoch makes the value unusable whatever its age
  (§4.4). The read loads in the foreground and answers `fresh` with the new
  value. Test `a value from an earlier epoch is not served as fresh`
  (`workers/test/source-do.test.ts:187-194`).
- **Upstream down:** the read loads in the foreground. On failure it answers
  `stale-error` with the earlier-epoch value, with no age or `staleIfError`
  limit (R7). Under backoff it answers `stale-error` at once with no load (R9).
  Test `an earlier epoch is still better than nothing when upstream is down`
  (`workers/test/source-do.test.ts:196-202`).
- **A load that crosses an epoch boundary** stores the epoch of its **start**
  (now_l), so the next read after the boundary sees a mismatch and loads again.
  The only epoch in the repository is the campus date for `bonapp-page`
  (`workers/src/sources/bonapp.ts:26`, `:80`). The cache-age tests make a fetch
  cross campus midnight (`workers/test/cache-age.test.ts:64-96`). They check how
  the router dates and keeps the response, not what is stored.
- The alarm is not scheduled at epoch boundaries (§5.2).

---

## 8. Value-dependent freshness (`ttlFor`)

- **Evaluated** once per successful load, as `spec.ttlFor(value, now_l)`, on the
  value `load` returned before JSON round-tripping
  (`workers/src/source-do.ts:208`).
- **Stored** in the `ttl` column of `entry`. If the spec has no `ttlFor`, `null`
  is stored (`?? null`), so every success clears any earlier override.
- **Overrides** `ttl` with `row.ttl ?? spec.ttl`, both in the read decision
  (`workers/src/source-do.ts:110`), which affects T and W, and in alarm
  scheduling (`workers/src/source-do.ts:247`). The stale window is therefore
  `[row.ttl, row.ttl + staleIfError)`.
- **Not clamped** by the generic code. A returned `0` is kept, because `??` only
  replaces null and undefined. The value would then never be fresh, and the next
  alarm would be `fetched_at + 0`. The only `ttlFor`, `athleticsFreshFor`, clamps
  itself to between `MINUTE` (60 s) and `SOURCE_TTL` (1 h). It returns 60 s while
  a game is under way or about to start, and otherwise the time until 5 minutes
  before the next kickoff (`workers/src/sources/athletics.ts:26-36`, `:96`).
- The athletics route also derives the client `max-age` from it
  (`workers/src/router.ts:257-265`). That is route behaviour, covered in §11.

---

## 9. Idle and purge

- **`last_read`** is set on insert and refreshed by a read only when
  `now_r - last_read > 10 min` (`workers/src/source-do.ts:101-103`). It can
  therefore lag the latest read by up to 10 minutes. R0 does not touch it.
  Loads and alarms never touch it.
- **Going quiet:** when the alarm fires with `now - last_read > 2 days`, it does
  nothing and leaves no alarm (`workers/src/source-do.ts:253`). If a load
  finishes with `now_s - last_read > 2 days`, `#schedule` deletes the alarm
  (`workers/src/source-do.ts:245`). Both tests use the stored
  `last_read`, which can be up to 10 minutes older than the latest real read
  (`workers/src/source-do.ts:101-103`). A source can therefore become eligible to
  go quiet as early as 2 days minus 10 minutes after it was last really read.
  Eligibility is not the moment it goes quiet: that happens when the next alarm
  fires or the next load finishes. `#schedule` applies the test after every load,
  however the load was started. After a load that a read started, `last_read` is
  at most 10 minutes plus the load's duration old, so in practice the `:245`
  branch is reached from alarm-started loads.
- **Waking up:** see §5.3.
- **`purge()`** (`workers/src/source-do.ts:268-272`) runs `DELETE FROM entry`
  and `DELETE FROM response`, then `deleteAlarm()`. It does not touch
  `#inflight`. The next read inserts a fresh row and loads (R10). Test `purge
  forgets the value and the alarm` (`workers/test/source-do.test.ts:215-222`).
  No route calls it. It is called only from tests, e.g.
  `workers/test/cache-age.test.ts:38`.
- **Purge during a load:** the load's `UPDATE`s match no row. `#schedule` then
  runs `this.#row()!.name` on `undefined` and throws a TypeError from the
  `finally`, which replaces the load's result. A reader waiting on that load
  re-reads the row (`:126`), gets `undefined`, and throws at `:129`. That
  exception crosses RPC. UNVERIFIED: derived by reading the code only. No test
  covers it.
- **`inspect()`** (`workers/src/source-do.ts:263-266`) returns every row field
  except `value`, or `{}` when there is no row. It is read-only and is called
  only from tests (`workers/test/conditional.test.ts:133`, `:140`).

---

## 10. Upstream request policy

`upstream(url, init)` (`workers/src/upstream.ts:11-18`) is the only path the
sources use. It applies to `StudentWorkDO`, `StudentOrgsDO` and `ArchiveDO` as
well, but those use `notingRetryAfter` and not the conditional cache (§10.5).

### 10.1 User agent and headers

The caller's headers are kept. `User-Agent` is always overwritten with
`ccc-server/2.0` (`workers/src/upstream.ts:12-13`). Tests: `says it is
ccc-server, and does not follow a redirect`, and `keeps the caller’s headers,
but not its user agent or redirect` (`workers/test/upstream.test.ts:17-31`).

### 10.2 Redirects

`redirect: 'manual'` always overrides the caller's setting
(`workers/src/upstream.ts:14`). A 3xx is returned to the source's `load` as it
came. Each source decides how to treat it, and most treat `!response.ok` as
failure (e.g. `workers/src/sources/bonapp.ts:72-74`).

### 10.3 Conditional requests

These apply only when the request runs inside a `conditional` context **and**
its method is GET (`workers/src/upstream.ts:15-17`). `SourceDO.#load` runs
`spec.load` inside that context with `remember: true`
(`workers/src/source-do.ts:192-200`). `record` runs outside it.

1. **Address key:** the SHA-256 hex of the full URL, query included
   (`workers/src/conditional.ts:56-59`, `:95`). Test `a kept answer is stored by
   a digest of its address, never the address`
   (`workers/test/conditional.test.ts:114-124`).
2. **Kept set:** loaded at the start of each load from the `response` table.
   Rows whose body is a string, or whose body cannot be gunzipped, are skipped
   (`workers/src/source-do.ts:133-157`).
3. **Request:** if the kept answer has an `etag`, `If-None-Match` is set to it.
   If it has a `last_modified`, `If-Modified-Since` is set to it
   (`workers/src/conditional.ts:96-99`). The header is sent only when the field
   is non-empty.
4. **Response 304 with a kept answer:** the address is marked used with the
   **same** kept entry, and the load receives a synthesized `200` whose body is
   the kept text and whose headers are the kept headers
   (`workers/src/conditional.ts:107-110`). From there the load continues as
   normal. If it succeeds, the value is written with `fetched_at = now_l` and
   the read answers `fresh`, so a 304 counts as a fresh read. Test `send the
   validators the last answer came with, and a 304 is the kept body`
   (`workers/test/conditional.test.ts:52-89`).
5. **Response 304 without a kept answer:** returned to the load unchanged
   (status 304) (`workers/src/conditional.ts:107`, `:120`). What happens next is
   up to the source.
6. **Response 200 with `ETag` or `Last-Modified`, while `remember` is true:**
   the body is read from a clone. If `body.length ≤ MAX_KEPT` (16 Mi code
   units), the address is marked used with
   `{etag, lastModified, headers (without set-cookie), body}`
   (`workers/src/conditional.ts:111-119`). Status codes other than 200 are
   never kept.
7. **Response without validators:** not kept, so the next load asks for the
   full answer. Test `an answer without validators is asked for in full next
   time` (`workers/test/conditional.test.ts:104-112`).
8. **Persisting (success only):** the table is replaced by this load's used
   set. Any entry whose gzip exceeds 1 MiB is dropped
   (`workers/src/source-do.ts:161-178`). An address that this load did not
   request, or that did not answer 200 with validators (or 304 with a kept
   answer), is forgotten. If one load requests the same address twice, the later
   one wins (`Map.set`). Test `a page too big for a row is kept compressed`
   (`workers/test/conditional.test.ts:91-102`) keeps a page over 3 MB, because
   its gzip is under 1 MiB.
9. **Failure inside the load:** the `response` table is left unchanged, so the
   next attempt uses the same validators (`workers/src/source-do.ts:201` is
   reached only when `spec.load` succeeds). A failure after that point can leave
   the table rewritten (see the write rule in §3).

### 10.4 `Retry-After`

- **Statuses:** only 429 and 503 (`workers/src/conditional.ts:102`). The header
  is read only for GETs that go through a context (`workers/src/upstream.ts:16`).
- **Parsing** (`workers/src/conditional.ts:78-83`): a missing header gives 0.
  After trimming, a value of digits only is seconds × 1000. Anything else is
  `Date.parse(value) − now`, where `now = clock.now()` at request time
  (`workers/src/upstream.ts:17`). A non-finite result gives 0. A finite result
  is clamped to `[0, 86 400 000]`. Test `reads seconds or a date, and caps a
  day` (`workers/test/conditional.test.ts:143-151`).
- **Aggregation:** `context.retryAfter = max(previous, wait)` over every 429 or
  503 in the load (`workers/src/conditional.ts:103-104`).
- **Effect:** it is used only when the load **fails**, as the lower bound of
  `wait` (`workers/src/source-do.ts:222-225`). A load that gets a 429 or 503 and
  still succeeds (for example, athletics treats a failed livestats read as
  absent, `workers/src/sources/athletics.ts:86`) ignores it. The minimum hold is
  the exponential step (≥ 30 s). The maximum is 24 h.

### 10.5 Interaction with the out-of-scope objects

`notingRetryAfter` (`workers/src/conditional.ts:64-75`) runs work under a
context with `remember: false` and an empty kept set. Conditional validators
are never sent, and nothing is kept. Only `Retry-After` is collected. It is used
by `workers/src/board-schedule.ts:97` (for StudentWork and StudentOrgs) and by
`workers/src/archive-do.ts:185`. The generic `record` hook feeds `ArchiveDO`
(§2).

---

## 11. Client-facing HTTP layer

### 11.1 Entrypoints

- **Default (uncached) entrypoint** (`workers/src/worker.ts:20-34`), with Workers
  Cache disabled (`wrangler.jsonc:8`). For each request:
  1. `routes(inner)` sends a GET to `ctx.exports.CachedRoutes.fetch(inner)`, so
     it goes through the cache. Any other method calls `route(inner, env)`
     directly (`workers/src/worker.ts:23-24`).
  2. `legacy(request, routes)` either answers by itself or rewrites a Node `/v1`
     path to the campus route and passes it on. If it returns undefined, the
     request goes to `routes(request)` (`workers/src/worker.ts:25`,
     `workers/src/legacy.ts:84-117`). Legacy answers `/`, `/ping` and
     `/v1/routes` directly, outside the cache (`workers/src/legacy.ts:92-98`).
  3. `withETag(request, answer)` (`workers/src/worker.ts:26`).
  4. Trace attributes (`workers/src/worker.ts:28-31`).
- **`CachedRoutes` entrypoint** (`workers/src/worker.ts:14-18`), with Workers
  Cache enabled (`wrangler.jsonc:9`). It calls `route(request, env)` only.
  README: the cache keys by path and query, never by host, and each deployed
  version starts with an empty cache (`workers/README.md:79-86`). UNVERIFIED:
  how Workers Cache uses `Cache-Control` to decide what to keep and for how
  long. That is platform behaviour, not in this repository.
- **ETags are computed outside the cache**, on every response the default
  entrypoint returns, so cached responses are stored without an ETag.
- **Dev origin:** when `devPagesOrigin(env)` is defined, `route` overwrites
  `Cache-Control` with `no-store` on every response, including failures
  (`workers/src/router.ts:359-365`).

### 11.2 ETag and 304 (`workers/src/etag.ts`)

- An ETag is added only when the method is `GET`, the status is `200`, and the
  response has no `ETag` yet (`workers/src/etag.ts:22-24`).
- Tag: `"` + base64url(SHA-256(body)[0..18]) + `"`, using `+`→`-` and `/`→`_`
  (`workers/src/etag.ts:6-10`). The result is always 24 characters inside the
  quotes and is never weak.
- `matches(ifNoneMatch, tag)` (`workers/src/etag.ts:14-19`): `null` gives false.
  `*` (trimmed) gives true. Otherwise the header is split on `,`, each candidate
  is trimmed with a leading `W/` removed, and each is compared to the tag
  (treated the same way) by string equality.
- On a match the result is a `304` with a null body. Its headers are the
  response's headers plus `ETag`, without `Content-Type` and `Content-Length`,
  so `Cache-Control` is kept (`workers/src/etag.ts:29-33`). Otherwise the
  response is rebuilt with the same body, status and statusText, plus `ETag`
  (`workers/src/etag.ts:34`).
- Tests: `workers/test/etag.test.ts:8-56`.

### 11.3 `Cache-Control` by outcome

Read state (from `fetchSource`) mapped to HTTP:

| Situation | Status | `Cache-Control` | Code |
|---|---|---|---|
| Generic `served(...)` routes (directory, streams, convos, athletics), any non-error state (`fresh`, `stale`, `stale-error`) | 200 | `public, max-age=600`. The state is **not** distinguished. | `workers/src/router.ts:239-247` |
| Same, read throws (`error`) | 502 | `public, max-age=60` | `workers/src/router.ts:248-251` |
| Athletics success | 200 | `max-age = min(600, athleticsFreshFor(scores, clock.now())/1000)`, which overrides the 600 | `workers/src/router.ts:257-265` |
| News and calendar success / error | 200 / 502 | 600 / 60 | `workers/src/router.ts:147-151`, `:177-181` |
| Schedules: calendar or `breaks=none` / hours / error | 200 / 200 / 502 | 600 / `min(600, secondsUntilMidnight)` / 60 | `workers/src/router.ts:225-233` |
| Food (menu or café), success | 200 | `min(600, secondsUntilCampusMidnight(now))`, where `now` is the request start for 5-day requests and the clock after the fetch for 1-day requests | `workers/src/router.ts:72-92` |
| Food, nothing stored and upstream failing (stand-in) | 200 | `public, max-age=60` | `workers/src/router.ts:93-107` |
| WordPress API passthrough, answer status 200, state `fresh` or `stale` | 200 | 600 | `workers/src/sources/wordpress-api.ts:146-149` |
| WordPress API, answer status 200, state `stale-error` | 200 | 60, plus `Cache-Status: ccc-server; hit; detail=stale` | `workers/src/sources/wordpress-api.ts:147-150` |
| WordPress API, stored non-200 answer (e.g. a 4xx) | as stored | none | `workers/src/sources/wordpress-api.ts:146`, `:157` |
| WordPress API, read throws | 502 | 60 | `workers/src/sources/wordpress-api.ts:139-142` |
| Debug `/bonapp/:id` route (exposes `state` and `fetchedAt` in the body) | 200 / 404 / 502 | none | `workers/src/router.ts:113-130`, `:508-509` |
| `/`, 404 "not found", 405 | as given | none | `workers/src/router.ts:372-374`, `:382`, `:511` |

The food-route tests are in `workers/test/cache-age.test.ts:52-102`.

Only the WordPress API route treats `stale-error` differently from
`fresh`/`stale`. Every other route answers a `stale-error` value with the same
lifetime as a fresh one.

---

## 12. Errors as values

- `SourceDO.get` returns `{state: 'error', error: string}` and does not throw
  when nothing usable is stored (`workers/src/define-source.ts:32-35`,
  `workers/src/source-do.ts:89`, `:130`). The stated reason: workerd reports
  every exception that crosses RPC as unhandled, even when the caller catches
  it, and an upstream being down is routine (`workers/src/define-source.ts:32-34`;
  README `workers/README.md:372-373`).
- `get` catches load rejections (`workers/src/source-do.ts:125-127`). `alarm`
  swallows them (`workers/src/source-do.ts:257-259`).
- The conversion back to an exception happens in the caller's isolate:
  `fetchSource` notes the state and then throws `new Error(result.error)`
  (`workers/src/client.ts:11-12`). Routes catch it and turn it into a 502 or a
  stand-in (§11.3).
- `get` can still throw across RPC in the purge-during-load edge case (§9), or
  when the stored `value` fails `JSON.parse` (`workers/src/source-do.ts:84`).
  UNVERIFIED whether the latter can happen.

---

## 13. Trace attributes

All are set through `note()`, which sets attributes on the active span if there
is one and otherwise does nothing (`workers/src/trace.ts:9-11`).

| Attribute | Set at | Values |
|---|---|---|
| span name `source <name>` | `workers/src/client.ts:8` (`inSpan`, `workers/src/trace.ts:15-17`) | One span per `fetchSource` call. |
| `ccc.source` | `workers/src/client.ts:11` | The source name (caller side). |
| `ccc.source.state` | `workers/src/client.ts:11` | `fresh`, `stale`, `stale-error` or `error`. Not set if the RPC call itself throws. |
| `ccc.source` | `workers/src/source-do.ts:254` | `row.name`, on the alarm span, only when the alarm goes on to refresh (not when idle). |
| `ccc.source.failures` | `workers/src/source-do.ts:232` | Integer ≥ 1, the failure count after incrementing. Set on every failed load. |
| `ccc.source.backoff_ms` | `workers/src/source-do.ts:232` | `wait` in ms (§6.3), between 30 000 and 86 400 000. |
| `ccc.upstream.retry_after_ms` | `workers/src/conditional.ts:105` | The **running maximum** `context.retryAfter` in ms (0 to 86 400 000), set after each 429 or 503 response, including when the value is 0. |
| `ccc.campus` | `workers/src/router.ts:383` | Campus prefix (e.g. `edu.stolaf`). |
| `ccc.legacy_host` | `workers/src/worker.ts:29` | Boolean. |
| `http.response.status_code` | `workers/src/worker.ts:30` | Final status after `withETag` (304 means an ETag match). |
| `ccc.sentry.resource`, `ccc.telegram.status_code` | `workers/src/sentry-telegram.ts:236`, `:251` | Out of scope. Listed only for completeness. |

---

## 14. Scenario index

### `workers/test/source-do.test.ts` (16 tests; source: `ttl` 1 h, `staleIfError` 1 day, `epoch` variable, `:18-25`)

| Test name (exact) | Line | Specified by |
|---|---|---|
| `a cold miss loads once and is fresh` | 52 | §4.2 R10 (success); §4.1 step 3 insert |
| `a read within the ttl does not load again` | 59 | §4.2 R1 |
| `concurrent cold misses share one upstream fetch` | 66 | §4.2 R10 + R11; §4.3 |
| `a stale read answers at once and refreshes from the alarm` | 84 | §4.2 R2 (`f = 0` → `stale`); §5.1 `setAlarm(now_r)`; §5.2; then R1 |
| `the alarm refreshes ahead of demand and schedules the next one` | 100 | §5.2 (alarm loads regardless of state); §5.1 `next = fetched_at + ttl`; R1 |
| `stored values survive eviction` | 112 | §5.3; §4.2 R1 |
| `with nothing stored, the read rejects and backs off` | 123 | §4.2 R10 (failure) then R12; §6.2; §12 |
| `within staleIfError, a value whose refresh failed is a stand-in` | 131 | §4.2 R2 (`stale`), §5.2 alarm failure (§6.2), then R3 (`stale-error`); §6.6 "within" |
| `past staleIfError, a failed refresh still serves the stored value` | 142 | §4.2 R4 (failure → `stale-error`); §6.6 "past" |
| `backoff suppresses upstream calls, then lets one through` | 152 | §4.2 R4 (failure), R6, R4 (success); §6.3 row 1; §6.4 |
| `backoff doubles with each consecutive failure` | 169 | §4.2 R10 (failure) / R12; §6.3 rows 1–2 |
| `a value from an earlier epoch is not served as fresh` | 187 | §4.2 R7 (success); §7 "upstream up" |
| `an earlier epoch is still better than nothing when upstream is down` | 196 | §4.2 R7 (failure); §7 "upstream down" |
| `a source nobody reads goes quiet` | 206 | §5.2 step 1; §9 going quiet |
| `purge forgets the value and the alarm` | 215 | §9 purge; then §4.2 R10 |
| `an unregistered source is reported, not run` | 224 | §4.2 R0; §12 |

### `workers/test/conditional.test.ts` (7 tests; source `ttl` 1 min, `staleIfError` 0, `:18-29`)

| Test name (exact) | Line | Specified by |
|---|---|---|
| `send the validators the last answer came with, and a 304 is the kept body` | 52 | §10.3 items 3, 4, 6, 8; §10.1; §4.2 R4 (success; `staleIfError = 0` per §4.4) |
| `a page too big for a row is kept compressed` | 91 | §10.3 items 6, 8; §1 `MAX_KEPT_COMPRESSED` |
| `an answer without validators is asked for in full next time` | 104 | §10.3 item 7 |
| `a kept answer is stored by a digest of its address, never the address` | 114 | §10.3 item 1; §3.2 |
| `a 429 is not asked again before the wait it names` | 128 | §10.4; §6.3 (`max` with `retryAfter`); §4.2 R10 failure |
| `a 503 without one backs off as any failure does` | 137 | §10.4 (missing header → 0); §6.3 row 1 |
| `reads seconds or a date, and caps a day` | 143 | §10.4 parsing |

### `workers/test/upstream.test.ts` (2 tests)

| Test name (exact) | Line | Specified by |
|---|---|---|
| `says it is ccc-server, and does not follow a redirect` | 17 | §10.1, §10.2 |
| `keeps the caller’s headers, but not its user agent or redirect` | 23 | §10.1, §10.2 |

### `workers/test/etag.test.ts` (5 tests + 1 table-driven test with 6 cases)

| Test name (exact) | Line | Specified by |
|---|---|---|
| `a successful response carries one, the same for the same body` | 9 | §11.2 tag format |
| `a request holding the current body gets a 304 with no body` | 18 | §11.2 match → 304 |
| `a request holding an old body gets the new one` | 26 | §11.2 no match |
| `an error carries none` | 32 | §11.2 (status ≠ 200) |
| `a 304 has no content type` | 38 | §11.2 (`*`; `Content-Type` removed) |
| `%s against %s is %s` (6 cases: `"abc"`, `W/"abc"`, `"x", "abc"`, `*` → true; `"abd"`, `null` → false) | 53 | §11.2 `matches` |

### `workers/test/cache-age.test.ts` (8 tests; food routes, not the generic cache)

| Test name (exact) | Line | Specified by |
|---|---|---|
| `is the time left in the campus day` | 12 | §11.3 food row (`secondsUntilCampusMidnight`, `workers/src/sources/bonapp.ts:41-45`) |
| `is a whole day just after campus midnight, and mid-afternoon` | 18 | same |
| `follows the campus clock in winter, when it is an hour behind` | 23 | same |
| `a menu is kept no longer than the campus day it is for` | 52 | §11.3 food success |
| `café info is kept no longer than the campus day it is for` | 58 | §11.3 food success |
| `a fetch that ends after campus midnight is dated and kept by the day it ended` | 73 | §11.3 food success (1-day: clock after fetch); §7 load crossing an epoch boundary |
| `five days asked for across campus midnight are dated and kept by the day they were asked on` | 82 | §11.3 food success (5-day: request start) |
| `a stand-in made after campus midnight is dated by the day it was made` | 92 | §11.3 food stand-in; §4.2 R10 failure |

Mapped: 39 of 39 test definitions (16 + 7 + 2 + 6 + 8, with the table-driven
`matches` test counted once). Counting each of its six cases separately gives 44
of 44. Every `source-do.test.ts`
test maps to at least one decision-table row.

Rows with **no test**: R5, R8 (joining a load from a stale-past-window or
epoch-mismatch read), R9 (epoch mismatch while backed off), the `"upstream
unavailable"` fallback, `ttlFor` (§8; athletics has its own route tests, not
part of this mapping), the idle branch in `#schedule` (`:245`), purge during a
load, a 304 with nothing kept, and failure when the load ran longer than its
backoff (§5.1).

---

## 15. Inputs and outputs of the decision logic

### (a) Read

- Stored entry fields: `name`, `params`, `value` (parsed only for the answer),
  `fetched_at`, `epoch`, `ttl`, `failures`, `backoff_until`, `last_error`,
  `last_read`. Whether a row exists at all (`workers/src/source-do.ts:79-81`,
  `:92`).
- Stored `response` rows: `key`, `etag`, `last_modified`, `headers`, `body`
  (gzipped) (`workers/src/source-do.ts:133-157`).
- The clock: now_r (`:90`), now_l (`:191`), now_s (`:244`), the alarm-time read
  (`:253`), and the per-request time used for `Retry-After` dates
  (`workers/src/upstream.ts:17`).
- The source's declared policy, looked up by name in the registry: whether it is
  registered, `ttl`, `staleIfError`, `epoch(now)`, `ttlFor(value, fetchedAt)`,
  `load`, `record` (`workers/src/source-do.ts:88`, `:190`, `:243`).
- The read request: `name` and `params` (params are used only when the row is
  inserted).
- Whether a load is in flight (`#inflight`, `:183`).
- The result of a load: the value, or an exception with its `String(err)`. Also
  the per-request upstream results seen through the conditional context: status
  (200, 304, 429, 503 are significant), `ETag`, `Last-Modified`,
  `Retry-After`, headers and body (`workers/src/conditional.ts:101-119`).
- The constants in §1.

### (b) Produced

- An answer: `fresh`, `stale` or `stale-error` with `{value, fetchedAt}`, or
  `error` with a string (`workers/src/define-source.ts:22-35`).
- Entry writes: insert (name, params, last_read); touch last_read; on success
  (value, fetched_at, epoch, ttl, failures = 0, backoff_until = 0,
  last_error = NULL); on failure (failures, backoff_until, last_error);
  purge (delete).
- `response` table writes: replace-all on success; delete on purge.
- Alarm changes: `setAlarm(now_r)` (R2); `setAlarm(next)` or `deleteAlarm()`
  after each load; `deleteAlarm()` on purge. When the alarm fires and returns
  early, it leaves no alarm.
- Fetch starts: a new `#load`, or joining the existing one.
- Outgoing upstream request headers: `User-Agent`, `If-None-Match`,
  `If-Modified-Since`, and `redirect: manual`.
- A synthesized 200 in place of a 304, handed to `load`.
- `record` calls under `waitUntil`.
- Trace attributes (§13) and log lines (`workers/src/source-do.ts:214`).

---

## 16. Open points

1. **README says a value is "kept a day if its host fails"; the code has no age
   limit.** `workers/README.md:76-77` (also `:15-16`) and the field comment at
   `workers/src/define-source.ts:12` describe `staleIfError` as limiting how
   long the old value is served on error. The code serves the stored value as
   `stale-error` at any age after a failed foreground load
   (`workers/src/source-do.ts:129`). The test at
   `workers/test/source-do.test.ts:142-150` confirms the code.
2. **README says "a menu does not outlive midnight"** (`workers/README.md:17-18`).
   An earlier-epoch value is still served as `stale-error` when upstream fails
   or is backed off (`workers/src/source-do.ts:129`;
   `workers/test/source-do.test.ts:196-202`).
3. **README says "a stale value is answered at once and refreshed from the
   alarm"** (`workers/README.md:13-14`). That holds only inside the
   `staleIfError` window. Past it, or on an epoch mismatch, the reader waits for
   a foreground load (`workers/src/source-do.ts:115-128`).
4. **README says a 429 or 503 `Retry-After` "holds the source off at least that
   long"** (`workers/README.md:355-356`). It applies only if the load as a whole
   fails (`workers/src/source-do.ts:219-225`). A load that tolerates the 429 or
   503 ignores it.
5. **README says "Successes are cacheable for ten minutes and errors for a
   minute"** (`workers/README.md:75-76`). Several error responses carry no
   `Cache-Control` (`workers/src/router.ts:382`, `:511`;
   `workers/src/sources/wordpress-api.ts:133`, `:146`). Food stand-ins are 200s
   with 60 s (`workers/src/router.ts:93-107`). Routes other than WordPress
   answer `stale-error` with 600 s (`workers/src/router.ts:239-247`).
6. **README says reads return a value and do not throw**
   (`workers/README.md:372-373`). A purge during a load, or a stored value that
   cannot be parsed, can still throw from `get` across RPC
   (`workers/src/source-do.ts:84`, `:126-129`, `:242-243`). UNVERIFIED: no test.
7. **The `"upstream unavailable"` fallback** (`workers/src/source-do.ts:130`) is
   reachable only if `#load` throws before its `try` (`:189-197`), which also
   skips recording the failure and scheduling. UNVERIFIED whether any real input
   reaches it. Settle it by checking whether `#keptResponses`
   (`:152` `JSON.parse`) or `registry[row.name]!` can throw there. (A missing
   spec at `:190` does not throw, because `spec` is first dereferenced inside
   the `try`.)
8. **Alarm timing semantics (platform).** UNVERIFIED: a past `setAlarm(next)`
   (possible after a failed load that ran longer than its backoff,
   `workers/src/source-do.ts:246-248`) fires at once; a `setAlarm` made while
   the alarm handler is running survives that handler's completion; an alarm is
   cleared once its handler returns successfully (the last is implied by
   `workers/test/source-do.test.ts:212`). Settle with Cloudflare's Durable Object
   alarm documentation or a workerd test.
9. **Workers Cache behaviour (platform).** How `CachedRoutes` responses are kept,
   keyed and expired, and whether `Cache-Control: max-age` controls it
   (`wrangler.jsonc:6-10`; `workers/README.md:79-86`). UNVERIFIED: not
   determinable from the repository.
10. **`Retry-After` values that are not plain integers but look numeric** (e.g.
    `"1.5"`) go to `Date.parse` (`workers/src/conditional.ts:81`), whose result
    for such strings depends on the engine. UNVERIFIED. Settle with a workerd
    test.
11. **`MAX_KEPT` unit.** The comment says "bigger than this"
    (`workers/src/conditional.ts:32`), but the check is on JavaScript string
    length, in UTF-16 code units (`workers/src/conditional.ts:115`), not bytes.
12. **A stored row whose source is no longer registered.** `get` answers R0
    without touching the row (`workers/src/source-do.ts:88-89`). A pending alarm
    still runs `#load`. `spec.load` throws a TypeError inside the `try`, which
    is recorded as a failure. `#schedule` then reads `spec.ttl` only if
    `backoff_until ≤ now_s` (`:246-247`). Otherwise the alarm keeps retrying
    with backoff until the row is idle. UNVERIFIED: no test.
13. **A load whose `value` is `undefined`.** `JSON.stringify(undefined)` is
    `undefined` (`workers/src/source-do.ts:205`). UNVERIFIED what the SQLite
    binding stores, and whether `fetched_at` non-null with `value` NULL then
    breaks `JSON.parse` at `:84`.
14. **README omits the `MAX_KEPT` (16 Mi) limit before compression**
    (`workers/README.md:352` mentions only "up to 1 MB compressed";
    `workers/src/conditional.ts:35`, `:115`).

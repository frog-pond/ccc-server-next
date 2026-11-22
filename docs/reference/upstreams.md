# Upstream services of the reference Worker

Reference: `frog-pond/ccc-server` at commit `54cf5862e4a09917e9bdc932fa1632fd1c3e0f49`.
Scope: the Cloudflare Worker (`workers/src/**`) and the `source/**` modules it imports.
Every citation is `path:line` relative to the reference repository root. Where code and
`workers/README.md` disagree, the code is described and the disagreement is listed in Part 5.

Common terms used below (defined once, cited here):

- **SourceDO lifecycle.** A `defineSource` source is read through `fetchSource`
  (`workers/src/client.ts:7-15`), which calls one `SourceDO` per `name:key`
  (`workers/src/client.ts:9`). A value is *fresh* while `age < ttl` (or the value's own
  `ttlFor`) and in the same epoch (`workers/src/source-do.ts:106-112`); between `ttl` and
  `ttl + staleIfError` it is answered as `stale` and refreshed from the alarm
  (`workers/src/source-do.ts:114-119`); otherwise a refresh runs before answering, and if
  it fails any stored value, of any age, is answered as `stale-error`
  (`workers/src/source-do.ts:121-129`). With nothing stored the read is
  `{state:'error'}` (`workers/src/source-do.ts:130`), which `fetchSource` throws
  (`workers/src/client.ts:12`). Failures back off 30 s doubling to 1 h, or longer if a
  `Retry-After` asked (`workers/src/source-do.ts:15-16`, `220-225`). A source unread for
  2 days stops being refreshed (`workers/src/source-do.ts:18`, `245`, `253`).
- **"502, kept briefly"** means the route answers `502 {"message": ...}` with
  `Cache-Control: public, max-age=60` (`workers/src/router.ts:239-252`,
  `workers/src/lifetimes.ts:6`). Successes carry `max-age=600`
  (`workers/src/lifetimes.ts:3`) unless noted.
- **SOURCE_TTL** is 1 h (`workers/src/lifetimes.ts:10`); every source below uses
  `staleIfError` = 1 day unless noted.
- **Archive** means the source's `record` hook writes each read into an `ArchiveDO`
  (`workers/src/define-source.ts:16-17`, `workers/src/source-do.ts:210-217`), whose alarm
  separately walks the feed's history backwards, 2 steps per run, 1 min apart, backing off
  1 min doubling to 1 h, honouring `Retry-After` (`workers/src/archive-do.ts:11-14`,
  `170-212`). The walk is disabled when `ARCHIVE_BACKFILL` is `"off"`
  (`workers/src/archive-do.ts:107-109`). A walk starts 1 min after the feed's first record
  or history read (`workers/src/archive-do.ts:112-122`).

---

## Part 1: upstreams

### 1. BonApp café pages (`*.cafebonappetit.com`)

- **Hosts / allow-list.** URL must be `https:` and hostname must match
  `/^[a-z]+\.cafebonappetit\.com$/`; otherwise `load` throws
  `"<url> is not a BonApp café page"` before any fetch
  (`workers/src/sources/bonapp.ts:49`, `66-69`). Configured URLs are in
  `workers/src/cafes.ts:4-12` (ids) and `workers/src/cafes.ts:16-26` (names).
- **Used by.** Source `bonapp-page` (`workers/src/sources/bonapp.ts:62-81`). Routes
  (both campuses share the café tables, `workers/src/campuses.ts:138-139`, `169-170`):
  `/food/menu/:cafeId`, `/food/cafe/:cafeId` (`workers/src/router.ts:473-485`),
  `/food/named/{menu,cafe}/:name` (`workers/src/router.ts:487-506`), and the debug view
  `/bonapp/:cafeId[?full=1]` (`workers/src/router.ts:113-130`, `508-509`).
- **Request.** `GET <café url>` through `upstream` (`workers/src/sources/bonapp.ts:71`).
  Without `days`, one request to the undated café URL
  (`workers/src/router.ts:74-77`). With `?days=5` (menu routes only; any other value or
  repeat is a 400, `workers/src/router.ts:475-481`, `493-499`), five requests, one per
  campus date, each URL rewritten to `<café path>/<YYYY-MM-DD>/`
  (`workers/src/router.ts:74`, `source/menus-bonapp/shape.ts:223-238`); each dated URL is
  its own SourceDO. The `cafe` kind uses only the first page
  (`workers/src/router.ts:87-91`). No query parameters or extra headers.
- **Authentication.** None.
- **Response.** HTML. The `Bamco.*` script assignments are extracted by
  `source/menus-bonapp/extract-bamco.ts#extractBamco` (`source/menus-bonapp/extract-bamco.ts:62-105`)
  and validated by `source/menus-bonapp/types-bonapp.ts#BamcoPageContentsSchema`
  (`source/menus-bonapp/types-bonapp.ts:108-119`) in `parseBonappPage`
  (`workers/src/sources/bonapp.ts:54-56`). Shaped by
  `source/menus-bonapp/shape.ts#menuFromPages` / `#cafeFrom`
  (`source/menus-bonapp/shape.ts:169-193`, `261-292`), text cleaned by
  `source/menus-bonapp/clean.ts` (`source/menus-bonapp/clean.ts:46-63`).
- **Freshness.** ttl 1 h, staleIfError 1 day, **epoch = campus date** in
  `America/Chicago` (`workers/src/sources/bonapp.ts:16-26`, `77-80`): a value loaded on an
  earlier campus date is never fresh nor served stale-while-revalidate; a failed refresh
  still answers it as `stale-error` (`workers/src/source-do.ts:108`, `129`). The route's
  `max-age` is `min(600, seconds until campus midnight)`
  (`workers/src/router.ts:85`, `workers/src/sources/bonapp.ts:41-45`).
- **Failure handling.** Non-2xx (including any 3xx, since redirects are not followed) is
  an error (`workers/src/sources/bonapp.ts:72-74`). A page with no `Bamco` assignment is
  a closed café (`null`) (`source/menus-bonapp/extract-bamco.ts:63-65`); a page with
  assignments that cannot all be read throws `BamcoFormatError`
  (`source/menus-bonapp/extract-bamco.ts:67-102`). With nothing stored, `/food/*` answers
  **200** with a stand-in (`CafeMenuWithError` or `CustomCafe`) and `max-age=60`
  (`workers/src/router.ts:93-107`); with `days=5`, a failed day becomes an error stand-in
  for that day only, and only all days failing falls to the stand-in
  (`source/menus-bonapp/shape.ts:257-266`). An unknown café id is a 400 listing ids
  (`workers/src/router.ts:56-61`); `/bonapp/:cafeId` answers 404/502
  (`workers/src/router.ts:115-129`).
- **Quirks.**
  - Extraction is regex-based on script text: one-line JSON for `menu_items`/`cor_icons`,
    `Bamco.dayparts['<id>'] = {...};` per daypart, and a JS object literal for
    `current_cafe` with only `name` and `id` (`source/menus-bonapp/extract-bamco.ts:24-29`).
    A whole-object `Bamco.dayparts =` assignment (other than the `|| {}` initialiser) is
    refused (`source/menus-bonapp/extract-bamco.ts:19-22`, `67-69`). The count of
    assignments read must equal the count written (`source/menus-bonapp/extract-bamco.ts:97-102`).
  - `tier` is sent as `1` but as `"2"`/`"3"` and is coerced
    (`source/menus-bonapp/types-bonapp.ts:9-11`); nutrition values may be strings or numbers
    (`source/menus-bonapp/types-bonapp.ts:30-31`); `cor_icons` may be an array (treated as
    empty) (`source/menus-bonapp/types-bonapp.ts:116`, `source/menus-bonapp/shape.ts:207`).
  - The undated page "shows the day it is when BonApp answers", so it is dated by the clock
    after the fetch; dated pages are dated by the request's start
    (`workers/src/router.ts:68-86`).
  - A café page is 1.5–3.5 MB of HTML (`workers/src/conditional.ts:32-34`).
  - Whether a Worker's egress can fetch the café page was "unproven until the first
    deploy" (`workers/README.md:366-367`).
- **Test material.** `workers/test/bonapp.test.ts`, `food.test.ts`, `named-food.test.ts`,
  `cache-age.test.ts`, `router.test.ts`; fixtures `workers/test/fixtures/stav-hall.html`,
  `stav-hall.cafe.json`, `stav-hall.menu.json`; `source/menus-bonapp/{extract-bamco,shape,clean,types}.test.ts`.

### 2. WordPress REST posts as news feeds (`wp/v2/posts`)

- **Hosts / allow-list.** Live source: `https:` and hostname in `wp.stolaf.edu`,
  `www.carleton.edu`, `www.olafmessenger.com`, `thecarletonian.com`, else throws without
  fetching (`workers/src/sources/wp-news.ts:33-38`, `48-51`). Archive walk: the same four
  plus `content.krlx.org` (`workers/src/archives/news.ts:11-17`, `69-72`).
- **Used by.** Source `wp-news` (`workers/src/sources/wp-news.ts:44-62`) and archive
  `news` (`workers/src/archives/news.ts:33-95`). Routes: `/news/stolaf` (both campuses) and
  `/news/carleton-now` (both campuses) (`workers/src/campuses.ts:110-111`, `123-129`,
  `workers/src/router.ts:385-388`), with `?before=` from the archive
  (`workers/src/router.ts:138-146`, `workers/src/campuses.ts:90-98`). `/news/krlx?before=`
  also reads this archive, keyed by KRLX's WordPress posts endpoint
  (`workers/src/sources/rss-news.ts:16-18`, `workers/src/campuses.ts:100-103`).
  The Messenger and Carletonian URLs exist (`workers/src/sources/wp-news.ts:23-29`) but the
  routes `/news/mess` and `/news/carletonian` answer a fixed `newsDown` notice and fetch
  nothing (`workers/src/campuses.ts:112-118`).
- **Request.** Live: one `GET`, fixed URLs
  `https://wp.stolaf.edu/wp-json/wp/v2/posts?per_page=10&_embed=true`
  (`workers/src/sources/wp-news.ts:14`) and
  `https://www.carleton.edu/news/wp-json/wp/v2/posts?per_page=10&_embed=true`
  (`workers/src/sources/wp-news.ts:18-19`). Archive walk: `GET <posts endpoint>?per_page=20&page=<n>&_embed=true`,
  one page per step, page number as cursor (`workers/src/archives/news.ts:7`, `73-80`).
- **Authentication.** None.
- **Response.** JSON array of posts; validated and shaped by
  `source/feeds/wp-json-shape.ts#feedItemsFrom` (schema `WpJsonFeedEntrySchema`,
  `source/feeds/wp-json-shape.ts:10-49`, `92-94`) into `source/feeds/types.ts#FeedItemSchema`.
- **Freshness.** ttl 1 h, staleIfError 1 day (`workers/src/sources/wp-news.ts:60-61`).
  Every read is recorded in the archive (`workers/src/sources/wp-news.ts:59`).
- **Failure handling.** Non-2xx is an error (`workers/src/sources/wp-news.ts:54-56`); a
  body that is not JSON or not a post array throws (`workers/src/sources/wp-news.ts:57`).
  No content-type check. Archive walk: `400` with body `code: rest_post_invalid_page_number`
  ends the walk; other non-2xx is an error; the walk ends when a page is empty or
  `page >= x-wp-totalpages` (`workers/src/archives/news.ts:82-93`). Route with nothing
  stored: 502, kept briefly (`workers/src/router.ts:148-151`).
- **Quirks.**
  - St. Olaf's WordPress blocks the Node server's IP but not a Worker's
    (`workers/src/sources/wp-news.ts:12-13`).
  - `_embedded.author[]` entries and `_embedded['wp:featuredmedia'][]` entries may be an
    embedded WordPress error object (`{code,message,data}`) instead of the record; the
    schema makes every field optional and the shape falls back to `"Unknown Author"` /
    no image (`source/feeds/wp-json-shape.ts:13-37`, `59-73`).
  - `date_gmt` has no zone; `Z` is appended unless it already has `Z` or `+`
    (`source/feeds/wp-json-shape.ts:79-83`).
  - Featured image prefers `media_details.sizes.medium_large.source_url`
    (`source/feeds/wp-json-shape.ts:68-71`).
  - The Messenger URL uses host `www.olafmessenger.com` here but `olafmessenger.com` in the
    paper API (section 3); see Part 5.
- **Test material.** `workers/test/news.test.ts` (fixtures `stolaf-posts.json`,
  `stolaf-news.json`), `carleton-news.test.ts` (`carleton-posts.json`, `carleton-now.json`),
  `redirects.test.ts`, `archive.test.ts`, `retired-news.test.ts`;
  `source/feeds/wp-json-shape.test.ts`.

### 3. Student papers' WordPress REST API, passed through (`/wp-json/wp/v2/*`)

- **Hosts / allow-list.** Only two sites, compared by exact `upstream` base:
  `https://olafmessenger.com/wp-json/wp/v2` (`source/ccci-stolaf-college/v1/mess-shape.ts:10`,
  `workers/src/sources/wordpress-api.ts:28`) and `https://thecarletonian.com/wp-json/wp/v2`
  (`workers/src/sources/wordpress-api.ts:31-34`); any other throws
  (`workers/src/sources/wordpress-api.ts:38`, `63-64`). Path and query are re-checked with
  `rulesFor` inside `load` (`workers/src/sources/wordpress-api.ts:66-70`).
- **Used by.** Source `wordpress-api` (`workers/src/sources/wordpress-api.ts:58-105`).
  Routes `/news/mess/wp/v2/:resource[/:id]` and `/news/carletonian/wp/v2/:resource[/:id]`
  on both campuses (`workers/src/campuses.ts:132-135`, `workers/src/router.ts:390-395`).
- **Request.** `GET <base>/<resource>[/<id>][?<query as the client sent it>]`
  (`workers/src/sources/wordpress-api.ts:71`). Allowed resources and parameters, each given
  at most once and matching a pattern, are in `RESOURCES`
  (`source/ccci-stolaf-college/v1/mess-shape.ts:56-99`, `108-130`): `posts` (list and
  `/:id`), `categories`, `media`, `staff_profile`, `staff_year`, `pages`. The query string
  is forwarded unchanged; the SourceDO key is the decoded, name-sorted query
  (`source/ccci-stolaf-college/v1/mess-shape.ts:169-172`), so differently spelled requests
  share one stored copy but the first spelling is what is sent upstream. One request per
  read. Paging is the client's own `page`; the route adds an RFC 8288 `Link` from
  `x-wp-totalpages` when the resource allows `page`
  (`workers/src/sources/wordpress-api.ts:151-155`, `source/ccci-stolaf-college/v1/mess-shape.ts:136-164`).
- **Authentication.** None.
- **Response.** JSON (any shape, only checked to parse). Stored as
  `{status, type, body, headers}` with headers `x-wp-total`, `x-wp-totalpages`
  (`workers/src/sources/wordpress-api.ts:44-50`, `91-96`,
  `source/ccc-lib/stored-headers.ts:2`, `source/ccci-stolaf-college/v1/mess-shape.ts:176`).
  No schema; no shape module beyond `mess-shape.ts`.
- **Freshness.** ttl 1 h, staleIfError 1 day (`workers/src/sources/wordpress-api.ts:102-103`).
  The per-resource `ttl` values in `Rules` (5 min / 1 h / 1 day,
  `source/ccci-stolaf-college/v1/mess-shape.ts:68-98`) are not read by the Worker, which
  only reads `verdict.rules.params` (`workers/src/sources/wordpress-api.ts:152`).
- **Failure handling.** An error (and nothing stored) when: content-type lacks `json`,
  status ≥ 500, or status in `{403, 408, 429}` (`source/ccci-stolaf-college/v1/mess-shape.ts:181-188`);
  any 3xx, even with a JSON type (`workers/src/sources/wordpress-api.ts:76-81`); body not
  JSON after BOM strip (`workers/src/sources/wordpress-api.ts:82-90`); serialised answer
  over 1,900,000 bytes (`workers/src/sources/wordpress-api.ts:17-20`, `97-99`). **Other 4xx
  are a successful value**: stored, kept for the ttl, and passed on with their status and
  no `Cache-Control` (`workers/src/sources/wordpress-api.ts:144-157`). A refused request
  (unknown resource/param) is 404/400 without fetching
  (`workers/src/sources/wordpress-api.ts:132-133`). Nothing stored and upstream failing:
  `502 {"message": "<paper> could not be reached for <path>"}`, max-age 60
  (`workers/src/sources/wordpress-api.ts:139-142`). A `stale-error` 200 gets max-age 60 and
  `Cache-Status: ccc-server; hit; detail=stale` (`workers/src/sources/wordpress-api.ts:147-150`).
- **Quirks.**
  - A leading U+FEFF (from a theme's PHP file) is stripped (`workers/src/sources/wordpress-api.ts:52`, `83-85`).
  - 403/408/429 are treated as this server's standing with the host (bot check, rate
    limit), not as an answer (`source/ccci-stolaf-college/v1/mess-shape.ts:178-181`).
  - README states both papers' Cloudflare bot challenges answer the Worker with 403
    (`workers/README.md:101-103`, `116`).
  - `withPage` puts `page` last and omits it for page 1, keeping `_fields` commas unencoded
    (`source/ccci-stolaf-college/v1/mess-shape.ts:132-143`).
- **Test material.** `workers/test/wordpress-api.test.ts`;
  `source/ccci-stolaf-college/v1/mess.test.ts` (Node route, same shape module).

### 4. KRLX RSS feed (`content.krlx.org`)

- **Hosts / allow-list.** `https:` and hostname `content.krlx.org`, else throws without
  fetching (`workers/src/sources/rss-news.ts:22`, `34-37`).
- **Used by.** Source `rss-news` (`workers/src/sources/rss-news.ts:30-57`); route
  `/news/krlx` on both campuses (`workers/src/campuses.ts:116`, `128`). History
  (`?before=`) comes from the WordPress archive of `https://content.krlx.org/wp-json/wp/v2/posts`
  (section 2; `workers/src/sources/rss-news.ts:16-18`, `51-54`).
- **Request.** `GET https://content.krlx.org/feed/` (`workers/src/sources/rss-news.ts:13`, `39`). One request.
- **Authentication.** None.
- **Response.** RSS/XML. Must contain `<rss` followed by whitespace or `>`
  (`workers/src/sources/rss-news.ts:44-48`), and must be well-formed XML (saxes check,
  `source/ccc-lib/dom.ts:26-50`). Shaped by `source/feeds/rss-shape.ts#feedItemsFromRss`
  (`source/feeds/rss-shape.ts:68-71`); no zod schema for the input, output validated by
  `FeedItemSchema` (`source/feeds/rss-shape.ts:55-64`).
- **Freshness.** ttl 1 h, staleIfError 1 day (`workers/src/sources/rss-news.ts:55-56`).
- **Failure handling.** Non-2xx, no `<rss`, or malformed XML is an error; an RSS document
  with no items is an empty feed (`workers/src/sources/rss-news.ts:40-49`). Nothing stored:
  502, kept briefly (`workers/src/router.ts:148-151`).
- **Quirks.** The `<rss` check exists because a bot-challenge or error page can parse as an
  item-less feed (`workers/src/sources/rss-news.ts:44-46`). `dc:creator` and
  `content:encoded` are read by qualified tag name (`source/feeds/rss-shape.ts:16-19`,
  `36-39`). History items are shaped from WordPress, not RSS (`workers/README.md:346-347`).
- **Test material.** `workers/test/carleton-news.test.ts` (fixtures `krlx-feed.xml`,
  `krlx.json`); `source/feeds/rss-shape.test.ts` (uses `workers/test/fixtures/carletonian-feed.xml`
  and `carletonian.json`, a feed no Worker source reads).

### 5. Plain iCal feed (City of Northfield)

- **Hosts / allow-list.** `fetchFrom(ICAL_HOSTS, ...)`: `https:` and hostname in
  `www.northfieldmn.gov`, `www.carleton.edu`, else throws
  `"<origin+path> is not a calendar this reads"` (`workers/src/sources/calendars.ts:45-56`, `66`).
- **Used by.** Source `calendar-ical` (`workers/src/sources/calendars.ts:74-86`); route
  `/calendar/northfield` on both campuses (`workers/src/calendars.ts:85-86`, `102`, `114-121`).
- **Request.** `GET https://www.northfieldmn.gov/common/modules/iCalendar/iCalendar.aspx?catID=41&feed=calendar`
  with header `accept: text/calendar` (`workers/src/calendars.ts:85-86`,
  `workers/src/sources/calendars.ts:67-68`). One request. Archive walk: the same single
  request, all events including past (`workers/src/archives/calendars.ts:86-95`).
- **Authentication.** None.
- **Response.** iCal. Body (trimmed) must start with `BEGIN:VCALENDAR`
  (`source/calendar/ical-shape.ts:72-80`); parsed with ical.js and shaped by
  `source/calendar/ical-shape.ts#eventsFromIcal` (`source/calendar/ical-shape.ts:84-110`)
  into `source/calendar/types.ts#EventSchema`.
- **Freshness.** ttl 1 h, staleIfError 1 day (`workers/src/sources/calendars.ts:83-84`).
  Only events ending after load time are kept (`source/calendar/ical-shape.ts:96-98`).
  Recorded with span `[load time, ∞)` (`workers/src/sources/calendars.ts:80-82`).
- **Failure handling.** Non-2xx is an error naming origin+path only
  (`workers/src/sources/calendars.ts:52-54`); a non-calendar body is an error
  (`source/calendar/ical-shape.ts:68-77`). Nothing stored: 502, kept briefly
  (`workers/src/router.ts:178-181`).
- **Quirks.** Time resolution: DATE values at UTC midnight; TZID resolved via moment-timezone;
  floating times in `X-WR-TIMEZONE`; unknown zones fall back to calendar zone then UTC
  (`source/calendar/ical-shape.ts:17-35`, `91`). `ORGANIZER;CN=` is entity-decoded without
  HTML parsing (`source/calendar/ical-shape.ts:41-45`). A retired feed typically answers
  200 with site HTML (`source/calendar/ical-shape.ts:68-71`).
- **Test material.** `workers/test/calendars.test.ts` (iCal test at line 158),
  `archive.test.ts`; `source/calendar/ical.test.ts`, `parse.test.ts`.

### 6. Carleton calendars: iCal feed plus HTML calendar page (`www.carleton.edu`)

- **Hosts / allow-list.** `ICAL_HOSTS` as in section 5, for both requests
  (`workers/src/sources/calendars.ts:66`, `103-104`).
- **Used by.** Source `calendar-carleton` (`workers/src/sources/calendars.ts:94-122`).
  Routes on both campuses: `/calendar/carleton`, `/calendar/upcoming-convos`,
  `/calendar/sumo-schedule` (`workers/src/calendars.ts:99`, `109-110`), and on Carleton
  `/convos/upcoming` (`workers/src/campuses.ts:174`, `workers/src/router.ts:402`).
- **Request.** Two requests in parallel per read (`workers/src/sources/calendars.ts:102-110`):
  1. Feed: `GET https://www.carleton.edu<path>?loadFeed=calendar&stamp=<stamp>` with
     `accept: text/calendar` (`workers/src/calendars.ts:58-59`); paths and stamps
     `/calendar/` `1714843628`, `/convocations/calendar/` `1714843936`,
     `/student/orgs/sumo/schedule/` `1714840383` (`workers/src/calendars.ts:72-77`).
  2. Page: `GET https://www.carleton.edu<path>?start_date=<today>&end_date=<today+1 month>`
     in campus dates (`source/calendar/carleton-images-shape.ts:53-64`).
  Archive walk: one read of the feed URL, past included (`workers/src/calendars.ts:68`,
  `workers/src/archives/calendars.ts:86-95`).
- **Authentication.** None.
- **Response.** Feed: iCal, shaped by `eventsFromIcal` with `maxEndDate` = one month out
  (`workers/src/sources/calendars.ts:111`). Page: HTML scraped by
  `source/calendar/carleton-images-shape.ts#eventImages`; joined by
  `#withEventImages` (`source/calendar/carleton-images-shape.ts:14-46`).
- **Freshness.** ttl 1 h, staleIfError 1 day; recorded with span `[load, campus date one
  month ahead)` (`workers/src/sources/calendars.ts:113-121`, `38-39`).
- **Failure handling.** Feed failure is an error. Page failure (any error, including
  non-2xx or a parse problem) is logged and yields no images
  (`workers/src/sources/calendars.ts:104-109`). Nothing stored: 502, kept briefly.
- **Quirks.**
  - Fragile selectors: `.event_list--item`, `picture a[href*="eId="]`, its `img[src]`;
    the image URL has its query stripped to get the original from the CDN
    (`source/calendar/carleton-images-shape.ts:11-31`).
  - Join key: the feed's DESCRIPTION ends with `/_c/<code>`; `/_c/undefined` (athletics
    events) means no code (`source/calendar/carleton-images-shape.ts:5-9`, `34-38`).
  - Feed times are floating, given in the calendar's `X-WR-TIMEZONE`
    (`source/calendar/ical-shape.ts:22-23`).
  - The pages list different default spans, hence the explicit dates
    (`source/calendar/carleton-images-shape.ts:48-52`).
  - The calendar page is 1.5–3.5 MB of HTML (`workers/src/conditional.ts:33-34`).
- **Test material.** `workers/test/calendars.test.ts` (lines 84-145; fixtures
  `workers/test/fixtures/carleton-sumo.ics`, `carleton-sumo-schedule.html`);
  `source/calendar/carleton-images.test.ts`; `source/calendar/fixtures/carleton-sumo.ics`,
  `carleton-sumo-schedule.html`.

### 7. Google Calendar API (`www.googleapis.com`)

- **Hosts / allow-list.** Only `www.googleapis.com`, `https:` (`workers/src/sources/calendars.ts:150`,
  `workers/src/archives/calendars.ts:117`).
- **Used by.** Source `calendar-google` (`workers/src/sources/calendars.ts:131-165`); route
  `/calendar/krlx-schedule` on both campuses, calendar id `krlxradio88.1@gmail.com`
  (`workers/src/calendars.ts:106`). Archive kind `google`.
- **Request.** `GET https://www.googleapis.com/calendar/v3/calendars/<encodeURIComponent(id)>/events`
  with `maxResults=50&orderBy=startTime&showDeleted=false&singleEvents=true&timeMin=<now ISO>`
  (`workers/src/sources/calendars.ts:126`, `138-148`). One request; no paging (only the
  next 50). Archive walk: same endpoint with `maxResults=250`, `timeMin = now − 365 days`,
  `timeMax = now`, `pageToken=<cursor>`, following `nextPageToken`
  (`workers/src/archives/calendars.ts:18-20`, `100-125`).
- **Authentication.** Secret `GOOGLE_CALENDAR_API_KEY`, sent as header `X-Goog-Api-Key`,
  never in the URL (`workers/src/sources/calendars.ts:128-136`, `150-152`). Absent: `load`
  throws `"GOOGLE_CALENDAR_API_KEY is not set"` before fetching
  (`workers/src/sources/calendars.ts:135-136`; archive `workers/src/archives/calendars.ts:101-102`),
  so the route is a 502 when nothing is stored.
- **Response.** JSON; validated by `source/calendar/google-shape.ts` (`GoogleCalendarResultSchema`,
  `source/calendar/google-shape.ts:11-27`), shaped by `#eventsFromGoogle`
  (`source/calendar/google-shape.ts:55-57`).
- **Freshness.** ttl 1 h, staleIfError 1 day. Recorded span ends 1 ms before the 50th
  event's start when 50 came back, else ∞ (`workers/src/sources/calendars.ts:155-161`).
- **Failure handling.** Non-2xx is an error; the message names origin and path only, so
  the key never appears (`workers/src/sources/calendars.ts:41-55`). Nothing stored: 502.
- **Quirks.** `start.date`/`end.date` (all-day) or `dateTime` with offset
  (`source/calendar/google-shape.ts:12-19`); all-day dates are parsed by `moment(...)` in
  the process's local zone (`source/calendar/google-shape.ts:31-32`). Description `<br>`
  becomes newline before HTML is stripped (`source/calendar/google-shape.ts:33-34`).
- **Test material.** `workers/test/calendars.test.ts` (lines 188-247);
  `source/calendar/google.test.ts`. The test config binds a placeholder key
  (`workers/vitest.config.ts:11-12`).

### 8. The Events Calendar ("Tribe"/TEC) REST API (`wp.stolaf.edu/calendar`)

- **Hosts / allow-list.** Every page, including each `next_rest_url`, must be `https:` on
  `wp.stolaf.edu` (`workers/src/sources/calendars.ts:231`, `workers/src/archives/calendars.ts:150`).
- **Used by.** Source `calendar-tec` (`workers/src/sources/calendars.ts:213-246`); route
  `/calendar/stolaf` on both campuses (`workers/src/calendars.ts:107-108`). Archive kind `tec`.
- **Request.** `GET https://wp.stolaf.edu/calendar/wp-json/tribe/events/v1/events?per_page=50&ends_after=<campus yesterday>&starts_before=<campus today + 1 month>`
  (`workers/src/sources/calendars.ts:217-222`), then each page's `next_rest_url` in turn
  (`workers/src/sources/calendars.ts:224-235`). Up to 10 pages (`TEC_MAX_PAGES`,
  `workers/src/sources/calendars.ts:204-206`); an 11th throws. Archive walk: one month per
  step (`ends_after = today − back months − 1 day`, `starts_before = today − (back−1)
  months`), stopping after 12 consecutive empty months; cursor `"<back> <empties>"`
  (`workers/src/archives/calendars.ts:21-23`, `126-160`).
- **Authentication.** None.
- **Response.** JSON. Page validated by `workers/src/sources/tec-shape.ts#TecPageSchema`
  (`workers/src/sources/tec-shape.ts:33-37`); events by `TecEventSchema` and shaped by
  `#eventsFromTec` (`workers/src/sources/tec-shape.ts:18-29`, `77-89`).
- **Freshness.** ttl 1 h, staleIfError 1 day; recorded span `[load, campus date a month
  ahead)` (`workers/src/sources/calendars.ts:238-244`).
- **Failure handling.** Any page non-2xx, a page failing `TecPageSchema`, or more than 10
  pages is an error (`workers/src/sources/calendars.ts:227-232`). Events failing their
  schema are dropped individually; all of a non-empty list failing is an error
  (`workers/src/sources/tec-shape.ts:73-84`). Nothing stored: 502.
- **Quirks.**
  - `ends_after`/`starts_before` are rounded up by TEC to 23:59:59 of the date given, so
    yesterday's date is used to reach events ending early today; `start_date` is avoided
    because it drops ongoing exhibitions (`workers/src/sources/calendars.ts:208-212`).
  - `utc_start_date` is `"YYYY-MM-DD HH:mm:ss"`, UTC, no zone marker
    (`workers/src/sources/tec-shape.ts:39-40`).
  - `venue` is an object, or `[]` when there is none (`workers/src/sources/tec-shape.ts:11-13`);
    `organizer` is always an array (`workers/src/sources/tec-shape.ts:15-16`).
  - Free text is HTML-escaped (e.g. `&#8217;`) and decoded
    (`workers/src/sources/tec-shape.ts:42-43`).
- **Test material.** `workers/test/calendars.test.ts` (lines 429-520).

### 9. Presence (`api.presence.io`): events and student organisations

- **Hosts / allow-list.** Events: `fetchFrom(new Set(['api.presence.io']))`
  (`workers/src/sources/calendars.ts:192`, `workers/src/archives/calendars.ts:97`). Orgs:
  `https:` and origin exactly `https://api.presence.io`, else throws
  (`workers/src/student-orgs-do.ts:43`, `54-58`).
- **Used by.**
  - Source `calendar-presence` (`workers/src/sources/calendars.ts:188-200`); route
    `/calendar/student-orgs` on St. Olaf only (`workers/src/calendars.ts:114-118`).
  - Durable Object `StudentOrgsDO`, binding `STUDENT_ORGS`, one object named `stolaf`
    (`workers/src/student-orgs.ts:25`); routes on St. Olaf only: `/orgs`,
    `/orgs/categories`, `/orgs/uri/:uri` (`workers/src/campuses.ts:164`,
    `workers/src/router.ts:462-469`). Carleton `/orgs` is a fixed notice with no fetch
    (`workers/src/student-orgs.ts:48`, `source/ccci-carleton-college/v1/deprecated.ts:6-10`).
- **Request.**
  - Events: `GET https://api.presence.io/stolaf/v1/events`, one request, full list
    (`workers/src/calendars.ts:117`). Archive walk: the same single request
    (`workers/src/archives/calendars.ts:96-99`).
  - Org list refresh: three parallel `GET`s: `/stolaf/v1/organizations`,
    `/stolaf/v1/app/campus`, `/stolaf/v1/organizations/categories`
    (`source/student-orgs/presence-shape.ts:26-34`, `workers/src/student-orgs-do.ts:297-311`).
  - Org detail, on demand when someone opens an org:
    `GET /stolaf/v1/grid/portal-view/Organization/<encodeURIComponent(uri)>/`
    (`source/student-orgs/presence-shape.ts:37-38`, `workers/src/student-orgs-do.ts:195-221`).
    The uri must match `/^[a-z0-9]+(?:-[a-z0-9]+)*$/` and be ≤ 100 chars or the route is a
    404 without fetching (`workers/src/student-orgs.ts:63-69`), and must be in the stored
    list (`workers/src/student-orgs-do.ts:155-156`).
- **Authentication.** None.
- **Response.** JSON.
  - Events: `workers/src/sources/presence-shape.ts#eventsFromPresence` (outer `z.array`,
    per-event `PresenceEventSchema`, `workers/src/sources/presence-shape.ts:11-24`, `76-91`).
  - Orgs: `source/student-orgs/presence-shape.ts#presenceOrgs` (`BasicPresenceOrgSchema`,
    `PresenceCampusSchema`), `#presenceCategories` (`PresenceCategoryMembershipSchema`),
    `#withoutDemoOrgs`, `#withoutDemoCategory`, `#orgDetail`
    (`source/student-orgs/presence-shape.ts:40-61`, `73`, `124-204`); portal fields by
    `source/student-orgs/portal.ts#portalFields` (`source/student-orgs/portal.ts:27-52`).
- **Freshness.**
  - Events: ttl 1 h, staleIfError 1 day; recorded span `[load, ∞)`
    (`workers/src/sources/calendars.ts:195-198`). Only events whose end is after load time
    are kept (`workers/src/sources/presence-shape.ts:88`).
  - Orgs (`BoardSchedule`): refreshed every 1 h + up to 10 min random jitter; failure
    backoff 5 min doubling, capped at 1 h, or `Retry-After` if longer; idle after 2 days
    without a read (`workers/src/student-orgs-do.ts:27-35`,
    `workers/src/board-schedule.ts:92-139`). Details refreshed in the background once older
    than 1 h; after a failed detail read, not retried for 5 min
    (`workers/src/student-orgs-do.ts:36-39`, `158-178`).
- **Failure handling.**
  - Events: non-2xx; body not an array; all events failing their schema
    (`workers/src/sources/presence-shape.ts:72-85`). Nothing stored: 502.
  - Orgs: non-2xx from any of the three is an error (`workers/src/student-orgs-do.ts:59-64`);
    an empty list replacing a non-empty stored list is an error
    (`workers/src/student-orgs-do.ts:240-243`). With nothing ever stored the routes answer
    502 kept briefly with the last error or `"Presence unavailable"`
    (`workers/src/board-schedule.ts:70-81`, `workers/src/student-orgs.ts:15`). A detail that
    cannot be read with nothing stored: 502 `"Presence unavailable for this org"`
    (`workers/src/student-orgs-do.ts:162-171`).
- **Quirks.**
  - Event times are ISO 8601 with `Z` (`workers/src/sources/presence-shape.ts:42`). Event
    cover images are built from a hard-coded CDN prefix including St. Olaf's campus id
    (`workers/src/sources/presence-shape.ts:30-39`); org cover images use the `cdn` and
    `apiId` from `/app/campus` (`source/student-orgs/presence-shape.ts:76-85`).
  - Category memberships come flat, one row per org-category pair
    (`source/student-orgs/presence-shape.ts:130-137`).
  - Orgs in the `Demo` category are removed (`source/student-orgs/presence-shape.ts:124-126`, `154-167`).
  - The portal view is ~1.5 MB and includes non-public fields; only ten fields, by label,
    are kept (`source/student-orgs/portal.ts:7-23`).
  - The detail failure backoff is an in-memory `Map`, not stored
    (`workers/src/student-orgs-do.ts:73-74`).
  - Presence publishes no last-edited date; a constant `2000-01-01` is sent
    (`source/student-orgs/presence-shape.ts:64-70`).
- **Test material.** `workers/test/calendars.test.ts` (lines 308-377);
  `workers/test/student-orgs.test.ts` (fixtures `presence-orgs.json`,
  `presence-categories.json`, `presence-portal.json`);
  `source/student-orgs/{presence,portal,names}.test.ts`.

### 10. Oracle Recruiting (St. Olaf student jobs)

- **Hosts / allow-list.** `https:` and origin exactly
  `https://fa-ewur-saasfaprod1.fa.ocs.oraclecloud.com`, else throws
  (`source/student-work/oracle-shape.ts:5`, `workers/src/student-work-do.ts:93-99`).
- **Used by.** Durable Object `StudentWorkDO`, binding `STUDENT_WORK`, object `stolaf`
  (`workers/src/student-work.ts:76`). Routes on St. Olaf only:
  `/student-work/postings`, `/student-work/postings/:id` (id `\d{1,12}`), `/student-work/units`
  (`workers/src/campuses.ts:163`, `workers/src/router.ts:449-455`). St. Olaf `/jobs` is a
  fixed notice (`workers/src/campuses.ts:165`, `workers/src/student-orgs.ts:83-86`).
- **Request.**
  - Board: `GET <origin>/hcmRestApi/resources/latest/recruitingCEJobRequisitions?onlyData=true&expand=requisitionList&finder=findReqs;siteNumber=CX_1,limit=200,sortBy=POSTING_DATES_DESC`,
    values percent-encoded by hand, not `URLSearchParams`
    (`source/student-work/oracle-shape.ts:6-29`).
  - Detail: `GET <API>/recruitingCEJobRequisitionDetails?onlyData=true&expand=all&finder=ById;Id=<id>,siteNumber=CX_1`
    (`source/student-work/oracle-shape.ts:31-41`).
  - Per run: one board request, then details for postings that are new, older than 1 day,
    or unit-less and older than 1 h, at most 20 per run, 4 concurrently; leftovers
    trigger a follow-up run 2 min later (`workers/src/student-work-do.ts:43-58`, `388-433`).
- **Authentication.** None.
- **Response.** JSON. `source/student-work/oracle-shape.ts#boardPostings` (`BoardSchema`,
  `items` min 1) and `#oracleDetail` (`DetailSchema`)
  (`source/student-work/oracle-shape.ts:57-84`, `103-143`); description read by
  `source/student-work/posting-shape.ts#readDescription` and
  `source/student-work/unit-number.ts#unitNumberOfDescription`; listing built by
  `workers/src/student-work-shape.ts#listingOf` (`workers/src/student-work-shape.ts:63-107`).
  Areas come from section 15.
- **Freshness (bespoke schedule).** Board refresh every 4 h + up to 30 min random jitter;
  failure backoff 5 min doubling, capped at 4 h, or `Retry-After` if longer; idle after 2
  days (`workers/src/student-work-do.ts:43-67`, `workers/src/board-schedule.ts:92-139`).
  A stored board past due is answered at once; an alarm is set if none
  (`workers/src/board-schedule.ts:82-85`).
- **Failure handling.** 403 or 429 raise `Refused`; other non-2xx an error
  (`workers/src/student-work-do.ts:100-107`). A board of zero postings replacing a
  non-empty stored board is an error (`workers/src/student-work-do.ts:326-330`). A detail
  failure leaves the posting as it was; a `Refused` detail stops the other workers and fails
  the run (backoff), after the board has already been saved
  (`workers/src/student-work-do.ts:403-430`). Nothing ever stored: 502 kept briefly with the
  last error or `"Oracle Recruiting unavailable"` (`workers/src/student-work-do.ts:88-91`,
  `workers/src/board-schedule.ts:70-81`). Posting not on the board: 404 max-age 60
  (`workers/src/student-work.ts:100`).
- **Quirks.** Oracle sits behind its own bot protection
  (`workers/src/student-work-do.ts:57`). `PostedDate` is `YYYY-MM-DD` with no zone
  (`source/student-work/oracle-shape.ts:52`). Ids must be 1-12 digits
  (`source/student-work/oracle-shape.ts:31-32`). Unit numbers may carry a fund or office
  prefix, zero-width characters, or name two units (first wins)
  (`source/student-work/unit-number.ts:3-31`). Requests inside a run are never conditional
  (`workers/src/board-schedule.ts:97`, `workers/src/conditional.ts:61-75`).
- **Test material.** `workers/test/student-work.test.ts` with
  `source/student-work/fixtures/detail-*.json` (11 files);
  `source/student-work/{unit-number,areas,units}.test.ts`.

### 11. Carleton Student Employment WordPress (`www.carleton.edu/student-employment`)

- **Hosts / allow-list.** Origin must equal `https://www.carleton.edu`
  (`source/student-work/carleton-shape.ts:9`, `workers/src/carleton-board.ts:59-62`).
- **Used by.** `StudentWorkDO` object `carleton` (the object keeps the first board kind it
  is asked for, `workers/src/student-work-do.ts:147-160`). Routes on Carleton:
  `/student-work/postings`, `/student-work/postings/:id`
  (`workers/src/router.ts:456-460`, `workers/src/carleton-student-work.ts:84-103`) and `/jobs`
  (`workers/src/student-orgs.ts:87-89`), sharing one read.
- **Request.** `GET https://www.carleton.edu/student-employment/post-jobs/wp-json/wp/v2/posts?per_page=100&page=<n>&_embed=wp:term`
  (`source/student-work/carleton-shape.ts:10-21`). Page 1, then pages 2..`x-wp-totalpages`
  sequentially, only while page 1 was full (100 posts); more than 10 pages is an error
  (`workers/src/carleton-board.ts:73-86`).
- **Authentication.** None.
- **Response.** JSON array validated by `source/student-work/carleton-shape.ts#CarletonJobPostSchema`
  (`workers/src/carleton-board.ts:68`), shaped by `#carletonPosting`
  (`source/student-work/carleton-shape.ts:126-157`); `/jobs` by
  `source/ccci-carleton-college/v1/jobs-shape.ts#jobFromPost` (`workers/src/carleton-board.ts:205-215`).
- **Freshness.** Same `BoardSchedule` timing as section 10 (`workers/src/student-work-do.ts:59-67`, `315-321`).
- **Failure handling.** Non-2xx is an error (`workers/src/carleton-board.ts:64-67`); zero
  jobs replacing a stored board is an error (`workers/src/carleton-board.ts:105-109`). Nothing
  stored: 502 with `"Carleton Student Employment unavailable"` or the last error
  (`workers/src/student-work-do.ts:88-91`).
- **Quirks.** Posts are deduplicated by id and those in category `Archived` dropped
  (`workers/src/carleton-board.ts:88-95`, `source/student-work/carleton-shape.ts:104-106`).
  `date_gmt`/`modified_gmt` are UTC without zone (`source/student-work/carleton-shape.ts:108-109`).
  Labelled lines are read from bold-prefixed runs (`source/student-work/posting-shape.ts:140-160`,
  `source/student-work/carleton-shape.ts:42-58`); "Date Open" `M/D/YYYY` becomes ISO
  (`source/student-work/carleton-shape.ts:111-117`). `x-wp-totalpages` missing or invalid
  reads as 1 (`workers/src/carleton-board.ts:69-70`).
- **Test material.** `workers/test/carleton-student-work.test.ts`,
  `workers/test/student-orgs.test.ts` (`/jobs`), fixture `carleton-student-jobs.json`;
  `source/ccci-carleton-college/v1/jobs.test.ts`.

### 12. College athletics sites (`athletics.stolaf.edu`, `athletics.carleton.edu`)

- **Hosts / allow-list.** `https:` and hostname in those two, else throws naming
  origin+path (`workers/src/sources/athletics.ts:38-40`, `50-54`).
- **Used by.** Source `athletics-scores` (`workers/src/sources/athletics.ts:79-99`); route
  `/athletics/scores` on both campuses (`workers/src/router.ts:416`, `257-265`); params
  `scoresUrl`, `teamName` (`Oles` / `Knights`) (`workers/src/campuses.ts:153-156`, `179-182`).
- **Request.** Three parallel `GET`s per read (`workers/src/sources/athletics.ts:84-88`):
  1. `https://<host>/services/scores_chris.aspx?format=json` (`workers/src/campuses.ts:154`, `180`);
  2. `https://<host>/services/livestats.ashx` (`source/athletics/shape.ts:182-187`);
  3. `https://<host>/services/responsive-calendar.ashx?type=events&sport=0&date=<M/D/YYYY of campus yesterday>`
     (`source/athletics/shape.ts:190-195`, `445-453`).
- **Authentication.** None.
- **Response.** JSON. Scores: `source/athletics/shape.ts#scoresFromFeeds` (`AthleticsResponseSchema`,
  `ScoreSchema`); livestats `LivestatsResponseSchema` (safeParse); calendar
  `CalendarResponseSchema`/`CalendarEventSchema` (safeParse) via `#yesterdaysGames`;
  merged by `#withYesterday` (`source/athletics/shape.ts:58-173`, `286-291`, `462-481`).
- **Freshness (value-dependent).** `ttlFor = athleticsFreshFor`: 1 min while a game is in
  play (`started`/`live`/`unofficial-final`, for at most a day after kickoff) or kicks off
  within 5 min; otherwise until 5 min before the next kickoff, at most 1 h, at least 1 min
  (`workers/src/sources/athletics.ts:23-36`, `95-97`, `source/athletics/shape.ts:240-267`).
  The route's `max-age` is `min(600, athleticsFreshFor(scores, now)/1000)`, computed with the
  current clock, not the fetch time (`workers/src/router.ts:257-264`).
- **Failure handling.** Scores feed non-2xx or invalid is an error; livestats and calendar
  failures (including non-JSON) are logged and treated as absent
  (`workers/src/sources/athletics.ts:65-70`, `86-87`); unparseable livestats/calendar bodies
  count as empty (`source/athletics/shape.ts:269-279`, `455-466`). Nothing stored: 502.
- **Quirks.** `date_utc` is `"M/D/YYYY h:mm:ss AM/PM"` in UTC; all-day games carry a bare
  date and have no kickoff (`source/athletics/shape.ts:39-56`, `197-208`). Calendar `date` is
  school-local with no offset and its scores are school-first; away fixtures are swapped to
  home-first (`source/athletics/shape.ts:137-141`, `354-364`). Status `C` (cancelled) is
  dropped (`source/athletics/shape.ts:281-290`). Midnight start in the calendar marks all-day
  (`source/athletics/shape.ts:389-390`). A game's result can stay unposted indefinitely
  (`source/athletics/shape.ts:250-253`).
- **Test material.** `workers/test/athletics.test.ts`; `source/athletics/index.test.ts`;
  `source/athletics/fixtures/**` (README at `source/athletics/fixtures/README.md:1-30`).

### 13. St. Olaf directory lists (`www.stolaf.edu/directory`)

- **Hosts / allow-list.** Exact URL set: `https://www.stolaf.edu/directory/departments?format=json`,
  `https://www.stolaf.edu/directory/majors?format=json`; else throws
  (`workers/src/sources/stolaf-directory.ts:10-13`, `26`).
- **Used by.** Source `stolaf-directory`; routes `/directory/departments`,
  `/directory/majors` on St. Olaf only (`workers/src/campuses.ts:157-160`, `183`,
  `workers/src/router.ts:418-419`).
- **Request.** One `GET` of the URL (`workers/src/sources/stolaf-directory.ts:28`).
- **Authentication.** None.
- **Response.** JSON, passed through unvalidated (`workers/src/sources/stolaf-directory.ts:32`). No shape module.
- **Freshness.** ttl 1 h, staleIfError 1 day.
- **Failure handling.** Non-2xx or non-JSON body is an error
  (`workers/src/sources/stolaf-directory.ts:29-32`). Nothing stored: 502.
- **Test material.** `workers/test/directory.test.ts`.

### 14. St. Olaf streaming collection (`www.stolaf.edu/multimedia/api/collection`)

- **Hosts / allow-list.** No host check; the URL is the fixed constant
  `https://www.stolaf.edu/multimedia/api/collection` and only its query varies
  (`workers/src/sources/streams.ts:14-15`, `36`; `workers/src/archives/streams.ts:79`).
- **Used by.** Source `streams` (`workers/src/sources/streams.ts:32-60`), archive `streams`
  (`workers/src/archives/streams.ts:14-88`). Routes on St. Olaf only:
  `/streams/upcoming`, `/streams/archived` (with `?before=`), `/streams/search`
  (`workers/src/campuses.ts:161`, `workers/src/router.ts:318-354`, `421-422`).
- **Request.** One `GET ?<query>` with parameters sorted by name
  (`workers/src/sources/streams.ts:19-25`): `class` (`current`/`archived`/`all`),
  `date_from`, `date_to`, `sort`, and for search `squery`, `category`, `count`, `offset`
  (`source/ccci-stolaf-college/v1/streams-shape.ts:78-98`). Upcoming: campus today to +2
  months; archived: −2 months to today; client `dateFrom`/`dateTo`/`sort` override
  (`source/ccci-stolaf-college/v1/streams-shape.ts:194-213`). Search: `query` required,
  `count` 1-200 default 50, `offset` default 0, plain-digit integers only
  (`source/ccci-stolaf-college/v1/streams-shape.ts:40-69`, `217-239`). Invalid parameters
  are a 400 without fetching (`workers/src/router.ts:321-323`, `347-352`). Archive walk:
  `class=all&sort=descending&count=100&offset=<n>` until a page is empty or
  `offset ≥ meta.available` (`workers/src/archives/streams.ts:6-7`, `71-87`).
- **Authentication.** None.
- **Response.** JSON `{results: [...], meta?: {available}}`, validated by
  `source/ccci-stolaf-college/v1/streams-shape.ts#StreamEntryCollection` and shaped by
  `#streamsFrom` (`source/ccci-stolaf-college/v1/streams-shape.ts:4-26`, `102-113`). The
  search route adds a `Link` header from `#pageLinks` (`workers/src/router.ts:327-338`).
- **Freshness.** ttl 1 h, staleIfError 1 day. For `class=current` without `squery` and with
  both dates, the recorded span is `[date_from 00:00Z + 1 day, date_to 00:00Z]`
  (`workers/src/sources/streams.ts:42-56`).
- **Failure handling.** Non-2xx is an error whose message never names the query
  (`workers/src/sources/streams.ts:37-39`); invalid body throws. Nothing stored: 502.
- **Quirks.** `starttime` is `"YYYY-MM-DD HH:mm"` in `America/Chicago`
  (`source/ccci-stolaf-college/v1/streams-shape.ts:109`). Upstream answers a blank search
  with everything (`source/ccci-stolaf-college/v1/streams-shape.ts:53`). A lone
  `date_from`, lone `date_to`, or neither is accepted by upstream for search
  (`source/ccci-stolaf-college/v1/streams-shape.ts:90-93`). `eid` type is unknown and stored
  as JSON (`workers/src/archives/streams.ts:20-21`). The record span uses UTC midnights of
  the campus dates (`workers/src/sources/streams.ts:51-53`).
- **Test material.** `workers/test/streams.test.ts`, `workers/test/archive.test.ts`.

### 15. Carleton convocations: calendar event page (HTML) and Podbean podcast (RSS)

- **Hosts / allow-list.** No host check; URLs are built from constants
  `https://www.carleton.edu/convocations/calendar/` and
  `https://feed.podbean.com/carletonconvos/feed.xml`
  (`source/ccci-carleton-college/v1/convos-shape.ts:9-12`, `workers/src/sources/convos.ts:19-21`).
  The event id must match `/^[A-Za-z0-9]{1,32}$/` (route 404 without fetching; load throws)
  (`workers/src/sources/convos.ts:17`, `36`, `workers/src/router.ts:431-434`).
- **Used by.** Sources `convo-detail` and `convos-archived`
  (`workers/src/sources/convos.ts:32-66`), archive `convos`
  (`workers/src/archives/convos.ts:16-54`). Routes on Carleton only:
  `/convos/upcoming/:id`, `/convos/archived` (with `?before=`)
  (`workers/src/campuses.ts:185`, `workers/src/router.ts:424-436`).
- **Request.** Detail: `GET https://www.carleton.edu/convocations/calendar/?eId=<id>`
  (`workers/src/sources/convos.ts:37`). Archived: `GET` of the podcast feed
  (`workers/src/sources/convos.ts:46-50`). Archive walk: one read of the podcast feed
  (`workers/src/archives/convos.ts:50-53`).
- **Authentication.** None.
- **Response.** Detail: HTML; `Content-Type` must match `/html/`
  (`workers/src/sources/convos.ts:23-24`, `38`); scraped by
  `source/ccci-carleton-college/v1/convos-shape.ts#upcomingFrom`
  (`source/ccci-carleton-college/v1/convos-shape.ts:40-63`). Podcast: `Content-Type` must
  match `/xml/` and body must contain `<rss` (`workers/src/sources/convos.ts:46-50`); shaped
  by `#archivedFrom` (latest 100) / `#allConvosFrom`
  (`source/ccci-carleton-college/v1/convos-shape.ts:65-74`).
- **Freshness.** ttl 1 h, staleIfError 1 day (`workers/src/sources/convos.ts:40-41`, `63-64`).
- **Failure handling.** Non-2xx, wrong content type, missing `.campus-calendar--event`
  (node `assert` throws), non-RSS or malformed XML are errors
  (`workers/src/sources/convos.ts:19-26`, `source/ccci-carleton-college/v1/convos-shape.ts:44-45`).
  Nothing stored: 502.
- **Quirks.** Fragile selectors: `.campus-calendar--event`, `.event_description`,
  `.single_event_image a[href]`, `.sponsorContactInfo`; relative links resolved against the
  calendar URL and converted to Markdown (`source/ccci-carleton-college/v1/convos-shape.ts:40-62`).
  `pubDate` is parsed by `moment(...)`; a missing `pubDate` gives `moment(undefined)`, the
  current time (`source/ccci-carleton-college/v1/convos-shape.ts:19`). The value is
  JSON-round-tripped so moments become strings (`workers/src/sources/convos.ts:59-60`).
- **Test material.** `workers/test/convos.test.ts` (inline bodies), `workers/test/archive.test.ts`.

### 16. GitHub Pages published files (`stolaf.dev`, `carls-app.github.io`) and the dev origin

- **Hosts / allow-list.**
  - `schedules`: `https:` and host `stolaf.dev`, or under the dev origin
    (`workers/src/sources/schedules.ts:13`, `21-28`).
  - `calendar-weekly-schedule`: `stolaf.dev` via `fetchFrom`, or the dev origin
    (`workers/src/sources/calendars.ts:173-176`).
  - `pages-json` (read only via `student-work-areas`): `https:` and host `stolaf.dev` or
    `carls-app.github.io`, or under the dev origin (`workers/src/sources/pages-json.ts:13`, `23-28`).
  - Dev origin: `PAGES_ORIGIN_OVERRIDE` must parse as `http:`/`https:`; "under" compares
    parsed origin and path prefix (`workers/src/pages-origin.ts:8-37`).
- **Used by.**
  - Source `schedules` (`workers/src/sources/schedules.ts:56-63`): routes `/spaces/hours`
    and `/breaks` on St. Olaf (`workers/src/router.ts:410-414`, `201-235`).
  - Source `calendar-weekly-schedule`: `/calendar/ksto-schedule` on both campuses
    (`workers/src/calendars.ts:105`).
  - Source `student-work-areas` (wraps `pagesJson.load`, `workers/src/student-work.ts:33-54`):
    feeds St. Olaf student-work routes (section 10).
  - Redirect-only (no fetch, `307` with `max-age=600`, `workers/src/router.ts:187-194`):
    every `files` and `redirects` entry (`workers/src/pages-routes.ts:16-82`) and
    `/images/:group/:name` to `https://stolaf.dev/AAO-React-Native/img/...`
    (`workers/src/router.ts:440-444`, `source/ccc-lib/images-shape.ts:6-20`).
- **Request.**
  - Schedules: two parallel `GET`s, `https://stolaf.dev/AAO-React-Native/building-hours.json`
    and `.../breaks.json` (`workers/src/pages-routes.ts:45-48`, `workers/src/sources/schedules.ts:43-46`).
  - Weekly schedule: `GET https://stolaf.dev/AAO-React-Native/ksto-schedule.json`.
  - Areas: `GET https://stolaf.dev/AAO-React-Native/student-work-areas.json`
    (`workers/src/pages-routes.ts:40`, `workers/src/campuses.ts:163`).
  - With a dev origin, these URLs are rebased onto it and read directly on every request,
    bypassing the SourceDO and conditional requests (`workers/src/router.ts:212-224`,
    `workers/src/calendars.ts:44-49`, `workers/src/student-work.ts:62-65`), and every
    response is `Cache-Control: no-store` (`workers/src/router.ts:356-365`).
- **Authentication.** None.
- **Response.** JSON.
  - Schedules: envelope `{data}` unwrapped by `source/schedules/publications.ts#publicationData`
    (`source/schedules/publications.ts:3-8`), checked and expanded by
    `source/schedules/resolve.ts#resolveScheduleResponses` (`source/schedules/resolve.ts:83-102`,
    schema in `source/schedules/parse.ts:6-46`); `/spaces/hours` applies
    `source/schedules/active.ts#hoursAt` at request time (`workers/src/router.ts:227-230`).
  - Weekly schedule: `{data: WeeklyScheduleSchema}` (`workers/src/sources/calendars.ts:177`,
    `source/calendar/weekly-schedule-shape.ts:28-32`), shaped by `#weeklyScheduleEvents`.
  - Areas: `{data: [{slug, units[]}]}` checked by `source/student-work/areas.ts#listedUnitsOf`
    (`source/student-work/areas.ts:10-17`, `workers/src/student-work.ts:36-50`).
- **Freshness.** All ttl 1 h, staleIfError 1 day (`workers/src/sources/schedules.ts:60-61`,
  `workers/src/sources/calendars.ts:180-181`, `workers/src/sources/pages-json.ts:35-36`,
  `workers/src/student-work.ts:51-52`). `/spaces/hours` (unless `?breaks=none`) has
  `max-age = min(600, seconds to midnight in the calendar's timezone)`
  (`workers/src/router.ts:226-230`).
- **Failure handling.** Non-2xx or non-JSON is an error. Schedules: either file failing,
  a missing envelope, or the pair not resolving (unknown break, cycle, missing template) is
  an error (`workers/src/sources/schedules.ts:50-55`, `source/schedules/resolve.ts:4-80`);
  `?breaks=` other than `none` is a 400 (`workers/src/router.ts:207-210`). Nothing stored:
  502. Areas failing is tolerated: postings list without areas, except an `area` filter,
  which is a 502 (`workers/src/student-work.ts:57-70`, `84-86`).
- **Quirks.** Carleton's `/spaces/hours` is a redirect, not a read
  (`workers/src/pages-routes.ts:71`). Weekly schedule slots are wall-clock hours in the
  schedule's timezone, `HH:00` only, 24:00 allowed as end
  (`source/calendar/weekly-schedule-shape.ts:10-32`, `40-50`).
- **Test material.** `workers/test/schedules.test.ts`, `dev-pages-origin.test.ts`,
  `pages-routes.test.ts`, `published-redirects.test.ts`, `student-work.test.ts`;
  `source/schedules/fixtures/{spaces,spaces-resolved,calendar,calendar-response}.json`;
  `source/schedules/{active,resolve}.test.ts`; `source/calendar/weekly-schedule.test.ts`;
  `source/student-work/areas.test.ts`.

### 17. GitHub REST API (`api.github.com`)

- **Hosts / allow-list.** No check; fixed URL (`workers/src/previews.ts:5`).
- **Used by.** Route `GET /previews` (not under a campus) (`workers/src/router.ts:375`,
  `workers/src/previews.ts:70-88`). No source or Durable Object.
- **Request.** `GET https://api.github.com/repos/frog-pond/ccc-server/pulls?state=open&per_page=100`
  with `Accept: application/vnd.github+json` and Cloudflare fetch options
  `cf: {cacheTtl: 900, cacheEverything: true}` (`workers/src/previews.ts:5`, `13`, `72-75`).
  One request, no paging beyond 100. Not conditional (no source context).
- **Authentication.** None (unauthenticated; comment notes 60 requests/hour,
  `workers/src/previews.ts:10-12`).
- **Response.** JSON array cast to `Pull[]`, no runtime validation
  (`workers/src/previews.ts:40-46`, `77`); mapped by `fromPull`/`previewUrl`
  (`workers/src/previews.ts:22-38`, `59-66`).
- **Freshness.** Edge cache 15 min via `cf.cacheTtl`; client `max-age=60`
  (`workers/src/previews.ts:13-17`, `78-80`).
- **Failure handling.** Non-2xx (including 3xx) or a throw is `502` with `max-age=60`
  (`workers/src/previews.ts:76`, `81-86`).
- **Quirks.** Preview hostnames are derived from branch names; names over 52 characters get
  `null` because Cloudflare shortens them with a hash (`workers/src/previews.ts:19-38`);
  fork PRs get `null` (`workers/src/previews.ts:64-65`).
- **Test material.** `workers/test/previews.test.ts`.

### 18. Telegram Bot API (`api.telegram.org`)

- **Hosts / allow-list.** Fixed URL; not sent through `upstream` (`workers/src/sentry-telegram.ts:190-208`).
- **Used by.** Route `POST /sentry/telegram` (section 19).
- **Request.** `POST https://api.telegram.org/bot<TELEGRAM_BOT_TOKEN>/sendMessage`,
  `Content-Type: application/json`, body `{chat_id, message_thread_id?, text, parse_mode:"HTML",
  link_preview_options:{is_disabled:true}}` (`workers/src/sentry-telegram.ts:191-203`). One
  request per posted webhook. Plain `fetch`: no `User-Agent` override and the runtime's
  default redirect handling.
- **Authentication.** `TELEGRAM_BOT_TOKEN` in the URL path (traced, per comment
  `workers/src/sentry-telegram.ts:187-189`); `TELEGRAM_CHAT_ID` and optional
  `TELEGRAM_TOPIC_ID` in the body. Missing token or chat id: route 503 naming the missing
  secrets (`workers/src/sentry-telegram.ts:218-225`); topic set but not digits: 503
  (`workers/src/sentry-telegram.ts:226-228`); topic absent: General topic.
- **Response.** Status only; body read only for logging on failure
  (`workers/src/sentry-telegram.ts:252-255`).
- **Freshness.** N/A.
- **Failure handling.** Network error: 502 `"Telegram could not be reached"`; non-2xx:
  502 `"Telegram answered <n>"` (`workers/src/sentry-telegram.ts:247-256`).
- **Quirks.** Telegram HTML mode escapes only `& < > "`; each part clipped (500 chars, links
  ≤ 1000 or dropped) to stay under 4096 (`workers/src/sentry-telegram.ts:9-16`, `44-71`).
- **Test material.** `workers/test/sentry-telegram.test.ts`.

### 19. Sentry webhooks (inbound)

- **Hosts / allow-list.** Inbound only; authenticity by signature.
- **Used by.** `POST /sentry/telegram`, matched before the method check and outside
  `CachedRoutes` (`workers/src/router.ts:370-371`, `workers/src/worker.ts:22-24`).
- **Request (as received).** Raw body; headers `Sentry-Hook-Signature` (hex HMAC-SHA256 of
  the body) and `Sentry-Hook-Resource` (`workers/src/sentry-telegram.ts:230-235`). Handled
  resources: `issue`, `event_alert`, `error`, `metric_alert`; others answer 204
  (`workers/src/sentry-telegram.ts:153-185`, `244-245`).
- **Authentication.** `SENTRY_CLIENT_SECRET` as HMAC key, compared with
  `crypto.subtle.verify` (`workers/src/sentry-telegram.ts:27-42`). Missing: 503 (checked
  before the signature) (`workers/src/sentry-telegram.ts:218-225`). Bad signature: 401.
- **Response.** JSON payload parsed loosely (`workers/src/sentry-telegram.ts:86-111`, `237-242`).
- **Failure handling.** Non-POST 405; non-JSON 400; Telegram failures 502; success or
  ignored 204 (`workers/src/sentry-telegram.ts:210-257`).
- **Test material.** `workers/test/sentry-telegram.test.ts`; setup prose in
  `workers/docs/sentry-telegram.md`.

---

## Part 2: cross-cutting request policy

- **User agent.** `upstream()` sets `User-Agent: ccc-server/2.0`, overriding any caller
  value (`workers/src/upstream.ts:5`, `12-13`). The Telegram call bypasses `upstream`
  (`workers/src/sentry-telegram.ts:193`).
- **Redirects.** Always `redirect: 'manual'`, overriding the caller (`workers/src/upstream.ts:14`);
  callers then treat a 3xx as non-OK.
- **Conditional requests.** Only GETs made while a `SourceDO` loads: kept `ETag` /
  `Last-Modified` are sent as `If-None-Match` / `If-Modified-Since`; a 304 is replaced by the
  kept 200 body (`workers/src/upstream.ts:15-17`, `workers/src/conditional.ts:88-121`).
  Kept only for 200s with a validator, body ≤ 16 MiB characters and ≤ 1 MiB gzipped, keyed by
  SHA-256 of the URL, `set-cookie` dropped, and only after a successful load
  (`workers/src/conditional.ts:35-37`, `56-59`, `113-119`, `workers/src/source-do.ts:159-178`, `201`).
  Board and archive requests run with `remember: false` and send no validators
  (`workers/src/conditional.ts:61-75`).
- **`Retry-After`.** Read on 429 and 503 inside any context, as seconds or HTTP date,
  clamped to [0, 24 h] (`workers/src/conditional.ts:51-53`, `77-83`, `102-106`); the
  longest one lengthens the backoff of the SourceDO, board or archive walk
  (`workers/src/source-do.ts:221-225`, `workers/src/board-schedule.ts:107-114`,
  `workers/src/archive-do.ts:201-203`). Requests outside a context (e.g. `/previews`, dev-origin
  reads, Presence org details) do not note it.

---

## Part 3: shape module index

"Pure" = no I/O, no clock, no environment. "Default clock" = a parameter defaults to
`new Date()`/`moment()`; the Worker callers pass a time from `clock.now()` unless noted.
Tests listed as *indirect* exercise the module through a Node wrapper or a Worker route.

| File | Exports (functions) | Input | Output | Test | Pure? |
|---|---|---|---|---|---|
| `source/menus-bonapp/extract-bamco.ts` | `extractBamco`, `unescapeJsString`, class `BamcoFormatError` | HTML string | `unknown` (Bamco record) or `undefined` | `source/menus-bonapp/extract-bamco.test.ts`, `workers/test/bonapp.test.ts` | Yes |
| `source/menus-bonapp/types-bonapp.ts` | `BamcoPageContentsSchema` | `unknown` | `BamcoPageContents \| undefined` | `source/menus-bonapp/types.test.ts` | Yes (schema) |
| `source/menus-bonapp/types.ts` | `CafeInfo*Schema`, `CafeMenu*Schema` | — | response types | `source/menus-bonapp/types.test.ts` | Yes (schema) |
| `source/menus-bonapp/shape.ts` | `CustomCafe`, `CafeMenuIsClosed`, `CafeMenuWithError`, `cafeFrom`, `menuFrom`, `menuDates`, `cafeUrlForDate`, `settled`, `menuFromPages` | `BamcoPageContents \| null`, date string | `CafeInfoResponseType`, `CafeMenuResponseType` | `source/menus-bonapp/shape.test.ts`, `workers/test/food.test.ts` | Yes; date is passed in (`source/menus-bonapp/shape.ts:12-14`) |
| `source/menus-bonapp/clean.ts` | `textOf`, `stationName`, `subStationName`, `itemLabel`, `itemDescription`, `cleanMenuItem`, `cleanDayPart` | strings / items | cleaned strings / items | `source/menus-bonapp/clean.test.ts` | Yes (`@frogpond/titlecase`, linkedom) |
| `source/feeds/wp-json-shape.ts` | `feedItemsFrom`, `convertWpJsonItemToStory`, `WpJsonFeedEntrySchema` | `unknown` (posts array) | `FeedItemType[]` | `source/feeds/wp-json-shape.test.ts`, `workers/test/news.test.ts` | Yes (moment parses an explicit-UTC string) |
| `source/feeds/rss-shape.ts` | `feedItemsFromRss`, `convertRssItemToStory` | XML string | `FeedItemType[]` | `source/feeds/rss-shape.test.ts`, `workers/test/carleton-news.test.ts` | Yes, except `moment(pubDate)` parsing of a free-form date string (`source/feeds/rss-shape.ts:29-32`) |
| `source/feeds/types.ts` | `FeedItemSchema` | — | `FeedItemType` | indirect | Yes (schema) |
| `source/feeds/deprecated.ts` | `deprecatedWpJson`, `retiredNnb`, `newsDown` | `now: Date \| null` | `FeedItemType[]` | `workers/test/retired-news.test.ts` | Default clock (`new Date()`, `source/feeds/deprecated.ts:9`, `26`, `44`); Worker passes `null` (`workers/src/campuses.ts:115-120`) |
| `source/ccc-lib/dom.ts` | `parseHtml`, `parseXml`, `htmlFragment`, `textFromHtml` | strings | DOM / text | `source/ccc-lib/dom.test.ts` | Yes (linkedom, saxes) |
| `source/ccc-lib/html-to-markdown.ts` | `htmlToMarkdown` | HTML string or element | Markdown | `source/ccc-lib/url.test.ts` | Yes (turndown) |
| `source/ccc-lib/url.ts` | `makeAbsoluteUrl` | URL strings | string | `source/ccc-lib/url.test.ts` | Yes (normalize-url) |
| `source/ccc-lib/images-shape.ts` | `isPublishedImage`, `imageUrl`, `IMAGE_GROUPS` | group, name | boolean / URL | `source/ccc-lib/images.test.ts`, `workers/test/published-redirects.test.ts` | Yes |
| `source/ccc-lib/stored-headers.ts` | constants only | — | — | indirect | Yes |
| `source/ccc-lib/constants.ts` | constants only | — | — | — | Yes |
| `source/ccc-lib/deprecated.ts` | constants only | — | — | — | Yes |
| `source/calendar/types.ts` | `EventSchema` | — | `EventType` | indirect | Yes (schema) |
| `source/calendar/ical-shape.ts` | `eventsFromIcal`, `parseCalendar` | iCal string, source, options, `now` | `EventType[]` | `source/calendar/ical.test.ts`, `workers/test/calendars.test.ts` | No: default clock (`now = moment()`, `source/calendar/ical-shape.ts:37`, `88`); reads moment-timezone's zone database (`source/calendar/ical-shape.ts:13-15`, `33-34`); `isBefore(now,'day')` uses the moment's zone (`source/calendar/ical-shape.ts:54`) |
| `source/calendar/google-shape.ts` | `eventsFromGoogle`, `convertGoogleEvents` | `unknown` body, `now` | `EventType[]` | `source/calendar/google.test.ts` (indirect), `workers/test/calendars.test.ts` | No: default clock (`source/calendar/google-shape.ts:29`, `55`); all-day dates parsed in the process's local zone (`source/calendar/google-shape.ts:31-32`) |
| `source/calendar/carleton-images-shape.ts` | `eventImages`, `eventCode`, `withEventImages`, `carletonSpan`, `carletonPage` | HTML, URL, events, `moment` | `Map`, `EventType[]`, `URL` | `source/calendar/carleton-images.test.ts` (indirect), `workers/test/calendars.test.ts` | No: `carletonSpan` default clock and tz database (`source/calendar/carleton-images-shape.ts:53-56`); others pure |
| `source/calendar/weekly-schedule-shape.ts` | `weeklyScheduleEvents`, `WeeklyScheduleSchema` | `WeeklySchedule`, `now` | `EventType[]` | `source/calendar/weekly-schedule.test.ts` (indirect), `workers/test/calendars.test.ts` | No: default clock, tz database (schema refines zone names, `source/calendar/weekly-schedule-shape.ts:30`, `51-52`) |
| `source/calendar/deprecated.ts` | `deprecatedEvents` | title, text, `now` | `EventType[]` | `source/calendar/deprecated.test.ts`, `workers/test/calendars.test.ts` | Default clock (`source/calendar/deprecated.ts:8`); Worker passes `clock.now()` (`workers/src/calendars.ts:81-83`) |
| `workers/src/sources/tec-shape.ts` | `eventsFromTec`, `TecPageSchema` | `unknown[]` events, `now` | `EventType[]` | `workers/test/calendars.test.ts` | No: default clock (`workers/src/sources/tec-shape.ts:77`) |
| `workers/src/sources/presence-shape.ts` | `eventsFromPresence` | `unknown`, `now` | `EventType[]` | `workers/test/calendars.test.ts` | No: default clock (`workers/src/sources/presence-shape.ts:76`); filters by `now` (`:88`) |
| `source/athletics/shape.ts` | `scoresFromFeeds`, `gameState`, `kickoffTime`, `needsFrequentRefresh`, `livestatsUrlFromScoresUrl`, `yesterdayCalendarUrl`, `yesterdaysGames`, `scoreFromCalendarEvent`, `withYesterday`, `ScoreSchema` | `unknown` bodies, `now: Date`, `School` | `Score[]`, URLs, boolean | `source/athletics/index.test.ts` (indirect), `workers/test/athletics.test.ts` | Clock passed in; reads the ICU timezone database via `Intl.DateTimeFormat` for `America/Chicago` (`source/athletics/shape.ts:295-340`) |
| `source/ccci-stolaf-college/v1/streams-shape.ts` | `streamsFrom`, `pageLinks`, `shiftMonths`, `upcomingWindow`, `upcomingRange`, `listParams`, `searchParams`, `chicagoToday`, schemas | `unknown` / query record, `now` | `{streams, available}`, params, Link string | `workers/test/streams.test.ts`, `workers/test/archive.test.ts` | No: default clock (`new Date()`, `source/ccci-stolaf-college/v1/streams-shape.ts:38`, `165`, `183`, `197`, `217`); tz database (`:109`, `:160`); Worker passes `clock.now()` (`workers/src/router.ts:320`) |
| `source/ccci-stolaf-college/v1/mess-shape.ts` | `rulesFor`, `withPage`, `paginationLinks`, `canonicalKey`, `isPassable`, constants | resource, id, `URLSearchParams`; status, type | `Verdict`, strings, boolean | `workers/test/wordpress-api.test.ts`, `source/ccci-stolaf-college/v1/mess.test.ts` (indirect) | Yes |
| `source/ccci-stolaf-college/v1/deprecated-shape.ts` | `deprecatedLinkGroups` | text | link groups | `workers/test/published-redirects.test.ts` | Yes |
| `source/ccci-carleton-college/v1/convos-shape.ts` | `upcomingFrom`, `allConvosFrom`, `archivedFrom` | HTML / XML string | `{images, content, sponsor}` / convos with `moment` dates | `workers/test/convos.test.ts` | No: `moment(pubDate)` with a missing `pubDate` is the current time (`source/ccci-carleton-college/v1/convos-shape.ts:19`); uses `node:assert` (`:5`, `:45`) |
| `source/ccci-carleton-college/v1/jobs-shape.ts` | `convertJobPost`, `jobFromPost`, `JobPostSchema` | `JobPost` / `StoredJobPost` | job object | `source/ccci-carleton-college/v1/jobs.test.ts` (indirect), `workers/test/student-orgs.test.ts` | Yes (`moment.utc` of given strings) |
| `source/ccci-carleton-college/v1/deprecated.ts` | `unavailableOrgs`, re-export `retiredNnb` | — | org array | `workers/test/student-orgs.test.ts` | Yes (`retiredNnb` as above) |
| `source/ccci-carleton-college/v1/orgs-shape.ts` | `CarletonStudentOrgSchema`, `SortableCarletonStudentOrgSchema` | — | types | indirect | Yes (schema) |
| `source/schedules/publications.ts` | `publicationData` | `unknown` | `unknown` (`data`) | `workers/test/schedules.test.ts` (indirect) | Yes |
| `source/schedules/parse.ts` | `parseScheduleData` | two `unknown` | `ScheduleData` | `source/ccci-stolaf-college/v1/schedules-routes.test.ts` (indirect) | Yes |
| `source/schedules/resolve.ts` | `resolveScheduleResponses` | calendar, spaces (`unknown`) | `{hours, calendar}` | `source/schedules/resolve.test.ts` | Yes |
| `source/schedules/active.ts` | `campusDate`, `secondsUntilMidnight`, `activeBreaks`, `hoursAt` | `now: number`, timezone, responses | strings, numbers, hours | `source/schedules/active.test.ts` | Clock passed in; reads ICU timezone database (`source/schedules/active.ts:13-24`) and caches formatters at module level (`:10-11`) |
| `source/schedules/types.ts` | types only | — | — | — | Yes |
| `source/student-orgs/presence-shape.ts` | `presenceOrgs`, `presenceCategories`, `groupCategories`, `cleanOrg`, `withoutDemoOrgs`, `withoutDemoCategory`, `orgDetail`, `presenceBase`, `presenceUrls`, `presencePortalUrl`, schemas | `unknown` bodies, portal fields | `SortableStudentOrgType[]`, `OrgCategoryType[]`, `DetailedStudentOrgType` | `source/student-orgs/presence.test.ts` (indirect), `workers/test/student-orgs.test.ts` | Yes |
| `source/student-orgs/portal.ts` | `portalFields`, `contactsOf`, `advisorsOf`, `instagramLinks`, `urlOrBlank`, `markdownOf` | `unknown` portal view / field map / HTML | field map, contacts, Markdown | `source/student-orgs/portal.test.ts` | Yes |
| `source/student-orgs/names.ts` | `sortableName`, `groupableName`, `sortOrgs` | strings / orgs | strings / sorted orgs | `source/student-orgs/names.test.ts` | Uses `toLocaleUpperCase()` with the runtime's default locale (`source/student-orgs/names.ts:22`) |
| `source/student-orgs/types.ts` | schemas | — | types | indirect | Yes (schema) |
| `source/student-work/oracle-shape.ts` | `boardUrl`, `detailUrl`, `jobPageUrl`, `query`, `boardPostings`, `oracleDetail`, constants | `unknown` bodies, id | `BoardPosting[]`, `OracleDetail`, URLs | `workers/test/student-work.test.ts` (indirect) | Yes |
| `source/student-work/posting-shape.ts` | `payCode`, `term`, `displayTitle`, `searchWords`, `readDescription`, `readDescriptionWith`, `LEVELS` | title / HTML strings | `PayCode`, `Term`, `Description` | `workers/test/student-work.test.ts` | Yes (module-level Turndown instance) |
| `source/student-work/unit-number.ts` | `unitNumber`, `unitNumberOfDescription` | strings | `string \| null` | `source/student-work/unit-number.test.ts` | Yes |
| `source/student-work/carleton-shape.ts` | `carletonJobsUrl`, `carletonPosting`, `isArchived`, `CarletonJobPostSchema`, constants | `CarletonJobPost` | `CarletonPostingWithDescription` | `workers/test/carleton-student-work.test.ts` (indirect) | Yes |
| `source/student-work/areas.ts` | `listedUnitsOf`, `groupUnits`, `OTHER_UNIT` | `unknown`, unit map | `Set<string> \| undefined`, record | `source/student-work/areas.test.ts` | Yes |
| `source/student-work/units.ts` | `postingUnits`, `mostlyNull` | callbacks, cache | units | `source/student-work/units.test.ts` | Not pure (async, calls injected I/O, module-level `WeakMap`); reached from the Worker only as a type import (`source/student-work/areas.ts:2`) |
| `source/student-work/retired-jobs.ts` | `deprecatedJobs`, `RETIRED_JOBS_TEXT` | text, `now` | jobs array | `workers/test/student-orgs.test.ts` | Default clock (`source/student-work/retired-jobs.ts:14`); `toLocaleDateString('en-US')` without a time zone uses the runtime's zone (`:26-30`); Worker passes a fixed date (`workers/src/student-orgs.ts:79`, `85`) |
| `workers/src/student-work-shape.ts` | `postingDetail`, `listingOf`, `searchText`, `ftsQuery`, `parseFilters`, `NONE` | Oracle detail/board, `URLSearchParams` | `Listing`, `Filters`, FTS query | `workers/test/student-work.test.ts`, `workers/test/carleton-student-work.test.ts` | Yes |

Worker-side helpers with ambient reads that shapes depend on: `campusToday` and
`secondsUntilCampusMidnight` use `Intl` with `America/Chicago`
(`workers/src/sources/bonapp.ts:14-45`); `campusMonthAhead`, `tec` and the calendar archive use
moment-timezone (`workers/src/sources/calendars.ts:34-39`, `217-222`,
`workers/src/archives/calendars.ts:130`); `asOf` recomputes `isOngoing` with `moment(now)`
in the process's zone (`workers/src/archives/calendars.ts:168-171`); `BoardSchedule` jitter
uses `Math.random()` (`workers/src/board-schedule.ts:137`).

---

## Part 4: secrets and bindings

| Name | Kind | Used by | When missing |
|---|---|---|---|
| `GOOGLE_CALENDAR_API_KEY` | secret (`workers/src/env.d.ts:4-5`) | `calendar-google` and its archive walk, header `X-Goog-Api-Key` (`workers/src/sources/calendars.ts:135-152`, `workers/src/archives/calendars.ts:101-118`) | Load throws `"GOOGLE_CALENDAR_API_KEY is not set"`; `/calendar/krlx-schedule` is a 502 with nothing stored; archive walk backs off |
| `ARCHIVE_BACKFILL` | variable (`workers/src/env.d.ts:6-7`) | `ArchiveDO` alarms (`workers/src/archive-do.ts:57`, `108`) | Walk runs (only `"off"` disables it) |
| `SENTRY_CLIENT_SECRET` | secret (`workers/src/env.d.ts:8-9`) | HMAC check (`workers/src/sentry-telegram.ts:222-232`) | `POST /sentry/telegram` is 503 listing it |
| `TELEGRAM_BOT_TOKEN` | secret (`workers/src/env.d.ts:10-11`) | Telegram URL path (`workers/src/sentry-telegram.ts:193`) | 503 listing it |
| `TELEGRAM_CHAT_ID` | secret (`workers/src/env.d.ts:12-13`) | Telegram body (`workers/src/sentry-telegram.ts:197`) | 503 listing it |
| `TELEGRAM_TOPIC_ID` | secret, optional (`workers/src/env.d.ts:14-15`) | Telegram `message_thread_id` (`workers/src/sentry-telegram.ts:191`, `198`) | Group's General topic; a non-digit value is a 503 (`workers/src/sentry-telegram.ts:226-228`) |
| `PAGES_ORIGIN_OVERRIDE` | variable, dev only (`workers/src/env.d.ts:16-17`) | `devPagesOrigin` (`workers/src/pages-origin.ts:8-23`) | Normal behaviour (published hosts, SourceDO caching) |
| `SOURCE` | Durable Object binding → `SourceDO` (`wrangler.jsonc:13`) | `fetchSource` (`workers/src/client.ts:9`) | UNVERIFIED (runtime error; settle by reading workerd behaviour for an undefined binding) |
| `STUDENT_WORK` | Durable Object binding → `StudentWorkDO` (`wrangler.jsonc:14`) | `workers/src/student-work.ts:76`, `workers/src/carleton-student-work.ts:80`, `workers/src/student-orgs.ts:87` | UNVERIFIED, as above |
| `STUDENT_ORGS` | Durable Object binding → `StudentOrgsDO` (`wrangler.jsonc:15`) | `workers/src/student-orgs.ts:25` | UNVERIFIED, as above |
| `ARCHIVE` | Durable Object binding → `ArchiveDO` (`wrangler.jsonc:16`) | `workers/src/archive.ts:52-53` | UNVERIFIED, as above |
| `ctx.exports.CachedRoutes` | entrypoint export with Workers Cache enabled (`wrangler.jsonc:7-10`) | GET requests (`workers/src/worker.ts:14-18`, `23-24`) | n/a (declared in config) |

The test config binds placeholder values for the six secrets/variables above (names only:
`workers/vitest.config.ts:10-20`). No fixture contains a real secret value that this review found.

---

## Part 5: open points

1. **README vs code, `/news/mess` and `/news/carletonian`.** README says they are read
   from the papers' WordPress (`workers/README.md:99-104`); the code answers a fixed
   `newsDown` notice and fetches nothing (`workers/src/campuses.ts:112-118`).
2. **README vs code, "kept a day".** README says a source is "kept a day if its host
   fails" (`workers/README.md:76-77`); the code answers any stored value, of any age, as
   `stale-error` whenever a refresh fails (`workers/src/source-do.ts:121-129`).
   `staleIfError` only bounds the stale-while-revalidate window.
3. **README vs code, every request through `upstream`.** README says every upstream
   request goes through `upstream` (`workers/README.md:349-350`); the Telegram call uses
   `fetch` directly (`workers/src/sentry-telegram.ts:193`).
4. **README vs code, moment-timezone.** README says not to import `moment-timezone` into
   the Worker (`workers/README.md:32-33`); `workers/src/sources/calendars.ts:1`,
   `workers/src/sources/tec-shape.ts:1`, `workers/src/sources/presence-shape.ts:2` and
   `workers/src/archives/calendars.ts:1` import it, and `workers/vitest.config.ts:24-28`
   configures for it.
5. **README vs config, Preview bindings.** README says `previews` repeats the `SOURCE`
   and `STUDENT_WORK` bindings (`workers/README.md:434-436`); `wrangler.jsonc:44-51`
   repeats all four.
6. **README vs code, Google key missing.** README says without the key "those calendars
   are a 502" (`workers/README.md:264-266`); with a stored value they are served
   `stale-error` (`workers/src/source-do.ts:129`).
7. **Messenger host mismatch.** The paper API uses `https://olafmessenger.com`
   (`source/ccci-stolaf-college/v1/mess-shape.ts:10`); the feed URL and both allow-lists use
   `www.olafmessenger.com` (`workers/src/sources/wp-news.ts:23-24`, `35`,
   `workers/src/archives/news.ts:14`). Redirects are not followed, so whichever host
   redirects to the other cannot be read. UNVERIFIED which host answers directly; settling
   it needs a request to the site (not made).
8. **Oversize values.** Only `wordpress-api` guards the ~2 MB SQLite row limit
   (`workers/src/sources/wordpress-api.ts:17-20`). For other SourceDO values, a write that
   exceeds it would throw inside `#load`'s `try` (`workers/src/source-do.ts:198-219`) and
   presumably count as a failure. UNVERIFIED; settle with a workerd test writing a >2 MB value.
9. **Validators from board/archive sites.** README says those sites "send no validators"
   (`workers/README.md:357-359`); the code simply never asks conditionally there
   (`workers/src/conditional.ts:61-75`). UNVERIFIED what the sites send.
10. **Carleton feed `stamp` values.** Fixed per path "as the Node server asks"
    (`workers/src/calendars.ts:72-77`); their meaning and whether they expire is not stated
    in code. UNVERIFIED.
11. **Process time zone.** `moment(...)` parses of Google all-day dates
    (`source/calendar/google-shape.ts:31-32`), `isBefore(now, 'day')` comparisons in the
    calendar shapes, `asOf` (`workers/src/archives/calendars.ts:168-171`) and
    `toLocaleDateString` (`source/student-work/retired-jobs.ts:26-30`) depend on the
    runtime's local zone. The code assumes nothing explicit; UNVERIFIED that workerd runs in
    UTC (settle from workerd documentation or a test asserting
    `new Date().getTimezoneOffset() === 0`).
12. **Convocation `pubDate` edge cases.** A missing `pubDate` becomes the current time
    (`source/ccci-carleton-college/v1/convos-shape.ts:19`); an unparseable one becomes an
    invalid moment whose JSON form, and the archive's `Date.parse` of it
    (`workers/src/archives/convos.ts:20`), are UNVERIFIED; settle with a unit test.
13. **Carleton jobs paging condition.** Pages 2..N are read only while *page 1* had 100
    posts (`workers/src/carleton-board.ts:84`); whether a later short page should stop the
    loop is not addressed in code. Fact recorded; intent UNVERIFIED.
14. **GitHub unauthenticated rate limit.** "60 times an hour" is a comment
    (`workers/src/previews.ts:12`); no code handles 403/429 from GitHub specially beyond a
    502, and no `Retry-After` is noted for this request. UNVERIFIED whether the edge cache
    (`cf.cacheTtl`) caches error responses.
15. **Unused test fixtures.** `workers/test/fixtures/carletonian.json` and
    `carletonian-feed.xml` are read only by `source/feeds/rss-shape.test.ts:13-14`, for a
    Carletonian RSS feed no Worker source reads.

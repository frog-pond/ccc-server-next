# Conversion fixtures from ccc-server

`REFERENCE_COMMIT` records the [ccc-server](https://github.com/frog-pond/ccc-server)
revision for this directory. Copied files retain their original paths and bytes.
To copy a file, read the committed blob with
`git -C "$(tools/reference-path)" show "<commit>:<path>"`. Review changes to the
fixtures and their expected outputs before updating the pin. The route inventory
has its own pin in `docs/reference/REFERENCE_COMMIT`.

| Conversion | Input → expected output | Reference test |
| --- | --- | --- |
| St. Olaf WordPress news | `stolaf-posts.json` → `stolaf-news.json` | `workers/test/news.test.ts` |
| Carleton Now | `carleton-posts.json` → `carleton-now.json` | `workers/test/carleton-news.test.ts` |
| KRLX RSS | `krlx-feed.xml` → `krlx.json` | `workers/test/carleton-news.test.ts` |
| Carletonian RSS | `carletonian-feed.xml` → `carletonian.json` | `source/feeds/rss-shape.test.ts` |
| BonApp café and menu | `stav-hall.html` → `stav-hall.cafe.json` and `stav-hall.menu.json` | `source/menus-bonapp/shape.test.ts` |
| Hours and breaks | `calendar.json` and `spaces.json` → `calendar-response.json` and `spaces-resolved.json`, all in `source/schedules/fixtures/` | `source/schedules/resolve.test.ts` |
| Athletics scores | `20260926T180044Z-scores.json` and `20260926T180044Z-livestats.json` in `source/athletics/fixtures/2026-09-26-home-games/` → `generated/athletics-scores.json` | `workers/test/athletics.test.ts` |
| Carleton SUMO calendar | `carleton-sumo.ics` and `carleton-sumo-schedule.html` → `generated/carleton-sumo.json` | `workers/test/calendars.test.ts` |

Unless specified, input and copied output names in the table are relative to
`workers/test/fixtures/`. The BonApp test passes `2026-09-22` to the conversion
and substitutes that date for `$DATE` in each expected output. The athletics
test substitutes `2030` for `2026` in both input files and passes
`2030-09-26T18:00:44Z` as the clock. The SUMO test makes the same year
substitution in the ICS only and passes `2030-10-09T12:00:00Z`.

## Generating an expected output

When the reference has a captured input but no complete output file, add a
case to [the generator](../../tools/reference-fixture.mjs) that calls the
reference's existing conversion functions. Pass fixture parameters such as
a date explicitly. The generator uses Node.js 26 or newer and prints JSON.
It reads from a temporary copy of the pinned commit, installs locked npm
dependencies there, and disables network access for the conversion. Set
`npm_config_offline=true` to also require a warm npm cache for installation.

```sh
tools/reference-fixture athletics-scores > /tmp/athletics-scores.json
diff -u fixtures/ccc-server/generated/athletics-scores.json /tmp/athletics-scores.json
```

## Testing a ported conversion

Put the Rust test in the crate that owns the conversion. Embed the input and
expected output with `include_str!` or `include_bytes!`, so the test has its
fixtures on both the host and Wasm. Parse the input into the conversion's
upstream type, call the same conversion as the route, serialize the app
response, and compare JSON values. This comparison ignores object key order
and whitespace while preserving array order, field presence and values.

For example, in the WordPress conversion test, with `Posts` and `Feed`
standing for the crate's upstream and app response types:

```rust,ignore
#[cfg_attr(
    all(target_arch = "wasm32", target_os = "unknown"),
    wasm_bindgen_test::wasm_bindgen_test
)]
#[cfg_attr(not(all(target_arch = "wasm32", target_os = "unknown")), test)]
fn stolaf_posts_match_the_reference() -> Result<(), Box<dyn std::error::Error>> {
    let input = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/ccc-server/workers/test/fixtures/stolaf-posts.json"
    ));
    let output = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/ccc-server/workers/test/fixtures/stolaf-news.json"
    ));

    let posts: Posts = serde_json::from_str(input)?;
    let feed = Feed::try_from(posts)?;
    let expected: serde_json::Value = serde_json::from_str(output)?;
    assert_eq!(serde_json::to_value(feed)?, expected);
    Ok(())
}
```

`mise run test` runs with Cargo offline. Conversion tests use the checked-in
fixtures directly. The generator is a separate maintenance command. Run
`mise run e2e` by hand to compare running servers.

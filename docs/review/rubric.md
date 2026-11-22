# Review rules

A change is reviewed against these rules, and each finding names the rule it breaks. The `arch-review` skill says how thorough a review a given change needs.

Formatting, and anything `cargo fmt` or `cargo clippy` already catch, is not reported. A finding is something the reviewer has checked in the code, with its `file:line`.

## T: Types

- **T1. What a site sends and what the apps receive are different types.** The first mirrors the site's data and derives `Deserialize` but not `Serialize`. The second is what the apps are sent and derives `Serialize`. One struct used in both directions is a finding. (For an example of what not to do, [`WpJsonPost` in the removed `ccc-types`](https://github.com/frog-pond/ccc-server-next/blob/68659b84f3b8206b619381b7d240b4e7f8749930/ccc-types/src/news.rs#L9-L10) is used both ways.)
- **T2. Conversion is `TryFrom` with a typed error.** The error enum has one case for each way the site's data can be unusable. No free functions returning `Option`, and no silent defaults for data that is required.
- **T3. `Option` means the site can leave the field out.** A field is optional only where the reference's schema marks it optional or its tests show it missing. Making every field `Option<String>` to be safe is a finding.
- **T4. Closed sets are enums, and identifiers from an open set are newtypes.** A campus is an enum. A café id is a newtype, not a bare `String` or `u32`.
- **T5. A type with a rule about its contents keeps its fields private** and has a constructor that checks the rule. Public fields are for plain data.
- **T6. No `serde_json::Value` in a schema**, except where the reference itself passes a site's value through untouched, and then inside a named newtype.
- **T7. Public items follow the Rust API Guidelines**: common traits derived where they make sense, conversions named `as_`, `to_` and `into_` correctly, `#[non_exhaustive]` on error enums that may grow, an enum in place of a `bool` parameter where it reads better, and documentation on every public item.
- **T8. A struct or an enum, whichever fits the data.** Fields that always exist together are a struct, and an `Option` of that struct if the group can be absent. Exactly one of several cases is an enum. Several `Option` fields of which one is expected to be set, or a `bool` that says whether another field means anything, should be an enum.
- **T9. A trait describes a capability that more than one type provides**, such as a cache backend or a way of fetching from a site. A trait with one implementation and no second one planned is a finding. Where each implementor has exactly one choice of a type, it is an associated type, not a generic parameter.

## D: Documentation

- **D1. Every public item is documented**: types, fields, variants, functions and modules. The documentation says what the item represents, not what its name already says.
- **D2. A type states its rules.** If a value has a rule about its contents, a unit or a range, the documentation says so. A type for a site's data says where its shape comes from, with a link to the reference's schema.
- **D3. A function that can fail has an `# Errors` section** saying when each error is returned. Anything that can still panic has a `# Panics` section.
- **D4. A module's documentation says where it sits**: what calls it and what it calls.

## F: Efficiency

- **F1. A response is parsed once.** It is not parsed into a generic value and then again into a type.
- **F2. A copy has a reason.** Cloning a response body or a collection to get past the borrow checker is a finding unless a comment says why it is needed.

## E: Errors

- **E1. Each library crate has its own error enum** implementing `std::error::Error` and keeping its cause available through `source()`.
- **E2. No `String` errors, `Box<dyn Error>` or catch-all error crates in library code.**
- **E3. No panics outside tests.** No `unwrap`, `expect`, `panic!`, `todo!()`, `unreachable!()` that input can reach, or indexing that can fail. (The removed BonApp handler [panics while handling a request](https://github.com/frog-pond/ccc-server-next/blob/68659b84f3b8206b619381b7d240b4e7f8749930/ccc-upstream-handlers/src/bonapp.rs#L27) if an environment variable is unset.)
- **E4. Errors become HTTP responses in one place.** A library error does not implement `IntoResponse` or name a status code. The crate with the entry point decides the response. (The removed upstream client [does the opposite](https://github.com/frog-pond/ccc-server-next/blob/68659b84f3b8206b619381b7d240b4e7f8749930/ccc-upstream-proxy/src/lib.rs#L101-L112).)

## S: Structure

- **S1. Dependencies point one way.** Crates holding schemas and conversions depend on no runtime, HTTP client or web framework. Libraries shared by both builds do not depend on `tokio`, `reqwest`, `axum` or `worker`. Only the two entry-point crates and the code specific to one build do.
- **S2. Shared libraries build for `wasm32-unknown-unknown`.**
- **S3. Libraries are handed what they need.** No environment reads, globals, `OnceLock` singletons, system clock or spawned tasks in a library.
- **S4. An outside resource is described, not improvised.** It has a name that never changes, parameters that can be stored, and how long its value stays fresh, taken from the reference. A background refresh is a name plus parameters.
- **S5. The rules for whether a cached value is still fresh are not written twice.** They come from the one crate that holds them, not from a route or from code specific to one build.
- **S6. Routes are thin.** A route reads its input, asks for the data and turns the result into a response. Conversion logic inside a route handler is a finding.
- **S7. A new crate has a reason.** It wraps something with its own dependencies or its own rules, such as one cache backend. A crate that only re-exports, or holds one function, is a finding.
- **S8. A change to `trifecta` names what needs it**: the route or the build.

## O: Knowing what went wrong

- **O1. Internal failures are recorded as structured events.** The cache being unavailable, a refresh failing, a stale value being served after a failed refresh, or an upstream being held off after it asked us to wait, each produce an event with a stable name and fields. None is swallowed, and none exists only as formatted text.
- **O2. Secrets never appear in events or errors**, including inside a site's URL. The reference sends the Google key as a header for this reason.
- **O3. Event and field names do not change casually.** Renaming one gets the more thorough review.

## P: Matching the reference

- **P1. Expected outputs come from the reference**, through its fixtures and tests, never from the author's sense of what looks right.
- **P2. A deliberate difference from the reference is agreed in an issue first** and then stated in the documentation. It is not a line in a pull request.
- **P3. Failures are part of what must match.** Status codes, the bodies sent when a site is down, `Cache-Control` lifetimes, and `ETag` and `Link` headers agree with the route's entry in `docs/reference/routes.toml`.
- **P4. Tests do not contact a live site.**

## What a finding looks like

```
[T3] crates/upstream-wordpress/src/post.rs:41
`title` is `Option<Rendered>`, but the reference's schema requires it
(source/feeds/wp-json-shape.ts:44) and no fixture leaves it out.
Consequence: a post without a title would be sent as null instead of failing conversion.
Suggest: make it required, and add a `MissingTitle` case only if a fixture shows one.
```

Give the rule, the place, the evidence, the consequence and a specific change. If you are not sure something is wrong, say what you would need to check instead of reporting it.

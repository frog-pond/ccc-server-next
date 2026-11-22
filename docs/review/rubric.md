# Review rules

A change is reviewed against these rules, and each finding names the rule it breaks. The `arch-review` skill says how thorough a review a given change needs.

Formatting, and anything that already fails `mise run check`, is not reported. Clippy's pedantic lints only warn for now, so they are reported. A finding is something the reviewer has checked in the code, with its `file:line`.

## T: Types

- **T1. What a site sends and what the apps receive are different types.** Of serde's traits, the first derives `Deserialize` only, and the second `Serialize` only. One struct used in both directions is a finding. The exception is a value that is cached and read back: where the reference caches the parsed upstream value and converts it per request (a BonApp page, converted with the campus date), that type derives both, and its documentation says it is cached. (For an example of what not to do, [`WpJsonPost` in the removed `ccc-types`](https://github.com/frog-pond/ccc-server-next/blob/68659b84f3b8206b619381b7d240b4e7f8749930/ccc-types/src/news.rs#L9-L10) is used both ways.)
- **T2. Conversions say whether they can fail.** `From` when the conversion cannot fail. `TryFrom` with its own error enum when it can fail and needs no other input. A named function taking the extra input (the campus date, the current time) when it needs one. The error enum has one case for each way the site's data can be unusable. No `Option` and no silent default in place of a typed error.
- **T3. `Option` means the site can leave the field out.** A field is optional only where the reference's schema marks it optional or nullable, or its tests show it missing. Making every field `Option<String>` to be safe is a finding. A derived `Option` accepts both a missing field and `null`; where the reference accepts only one, the documentation says so and a test shows it. `#[serde(default)]` is used only where the reference itself substitutes the same default.
- **T4. Closed sets are enums, and identifiers from an open set are newtypes.** A campus is an enum, and so is anything the reference checks against a fixed table. A posting id is a newtype, not a bare `String` or `u32`.
- **T5. A type with a rule about its contents keeps its fields private** and has a constructor that checks the rule. Public fields are for plain data. If such a type is deserialized, it goes through that constructor (`#[serde(try_from = "...")]`), so a derived `Deserialize` cannot skip the check.
- **T6. No `serde_json::Value` in a schema.** Where the reference passes a site's value through untouched, keep the original text (`Box<serde_json::value::RawValue>` or the bytes) inside a named newtype, so that the bytes, and so the `ETag`, do not change.
- **T7. Public items follow the Rust API Guidelines**: common traits derived where they make sense, conversions named `as_`, `to_` and `into_` correctly, no `#[non_exhaustive]` on types used only inside this workspace (an exhaustive `match` at the edge should break when a case is added), an enum in place of a `bool` parameter where it reads better, and documentation on every public item.
- **T8. A struct or an enum, whichever fits the data.** Fields that always exist together are a struct, and an `Option` of that struct if the group can be absent. Exactly one of several cases is an enum. Several `Option` fields of which one is expected to be set, or a `bool` that says whether another field means anything, should be an enum.
- **T9. A trait describes a capability that more than one type provides**, such as a cache backend or a way of fetching from a site. A trait with one implementation and no second one planned is a finding. Where each implementor has exactly one choice of a type, it is an associated type, not a generic parameter. A test double is not a second implementation. Where one value per call is enough, such as the current time, pass the value instead of a trait. Until the rearchitecture is done, this is applied loosely to Trifecta: an extension point that a planned route will use is not a finding.

## D: Documentation

- **D1. Every public item is documented**: types, fields, variants, functions and modules. The documentation says what the item represents, not what its name already says.
- **D2. A type states its rules.** If a value has a rule about its contents, a unit or a range, the documentation says so. A type for a site's data says where its shape comes from, with a link to the reference's schema.
- **D3. A function that can fail has an `# Errors` section** saying when each error is returned. Anything that can still panic has a `# Panics` section.
- **D4. A module's documentation says where it sits**: what calls it and what it calls.

## F: Efficiency

- **F1. A response is parsed once.** It is not parsed into a `serde_json::Value` and then again into a type. Serde's own buffering for a tagged enum is fine. `#[serde(untagged)]` is used only when the upstream gives no tag to choose by, because its errors do not say which field was wrong.
- **F2. A copy has a reason.** Cloning a response body or a collection to get past the borrow checker is a finding unless a comment says why it is needed. Cloning an `Arc` or `Bytes` is not a copy.
- **F3. A cached response that does not depend on the request is served from stored bytes.** It is not deserialized and serialized again on every request.

## E: Errors

- **E1. Each library crate has its own error enum** implementing `std::error::Error` and keeping its cause available through `source()`.
- **E2. No `String` errors, `Box<dyn Error>` or catch-all error crates in library code.**
- **E3. No panics outside tests.** The one exception is `todo!()` in a draft pull request whose types are being reviewed; a draft is never merged. No `unwrap`, `expect`, `panic!`, `todo!()`, `unreachable!()` that input can reach, or indexing that can fail. (The removed BonApp handler [panics while handling a request](https://github.com/frog-pond/ccc-server-next/blob/68659b84f3b8206b619381b7d240b4e7f8749930/ccc-upstream-handlers/src/bonapp.rs#L27) if an environment variable is unset.)
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

- **P0. Accept what the reference accepts, and produce what it produces.** Every input the reference accepts is accepted, and gives the same output. A more exact type changes how an accepted input is represented, never which inputs are accepted or what is sent for them. Where the two conflict, the reference wins and the type is changed.
- **P1. Expected outputs come from the reference**, through its fixtures and tests, never from the author's sense of what looks right.
- **P2. A deliberate difference from the reference is agreed in an issue first** and then stated in the documentation. It is not a line in a pull request.
- **P3. Failures are part of what must match.** Status codes, the bodies sent when a site is down, `Cache-Control` lifetimes, and `ETag` and `Link` headers agree with the route's entry in `docs/reference/routes.toml`.
- **P4. Tests do not contact a live site.**

## What a finding looks like

```
[T3] <crate>/src/post.rs:41
`title` is `Option<Rendered>`, but the reference's schema requires it
(source/feeds/wp-json-shape.ts:46) and no fixture leaves it out.
Consequence: a post without a title would be sent as null instead of failing conversion.
Suggest: make it required, and add a `MissingTitle` case only if a fixture shows one.
```

Give the rule, the place, the evidence, the consequence and a specific change. If you are not sure something is wrong, say what you would need to check instead of reporting it.

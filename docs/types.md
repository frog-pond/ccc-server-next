# Types for a route

Use the reference's schema, conversion, and fixtures to work out which input
the route accepts and which response the apps receive. Preserve both under
[rule P0](review/rubric.md#p-matching-the-reference).

## Where the types go

- **App responses** live in [`ccc-schema`](../ccc-schema), with modules grouped
  by the API data they describe, such as `news`. Both the native server and the
  Worker use these types. The crate depends on no upstream, runtime, or web
  framework and builds for `wasm32-unknown-unknown`.
- **Upstream responses** live beside the code that reads that upstream. Put
  the schema in the upstream's module or a submodule of it. A schema describes
  the fields the reference reads for that upstream, with a link to the
  reference's schema.
- **Conversions and their errors** live with the upstream code. That code
  depends on `ccc-schema` and constructs its response types. For example, the
  crate that defines `WordPressPost` can implement `TryFrom<WordPressPost>` for
  an app response type from `ccc-schema`. Conversions do no I/O and take any
  date or time they need as input.

## Type rules

These conventions apply the [type rules](review/rubric.md#t-types).

- **T1** — Derive `Deserialize` for upstream types and `Serialize` for app
  response types. An upstream type that the reference caches and reads back
  as a parsed value derives both. Document that caching use and keep the app
  response a separate type.
- **T2** — Use `From` for a conversion that always succeeds and `TryFrom` for
  one that can fail with only the source value as input. Use a named function
  when the conversion needs additional input, such as the campus date. Give
  fallible conversions an error enum with a variant for each failure.
- **T3** — Use `Option` where the reference's schema or tests show that a field
  can be missing or null. If the reference accepts only one of those cases,
  preserve that distinction and document and test it. Apply defaults where
  the reference applies the same defaults.
- **T4** — Use enums for closed sets and newtypes for identifiers from open
  sets.
- **T5** — Keep fields with an invariant private. Check it in a constructor
  and deserialize through that check with `#[serde(try_from = "...")]`.
- **T6** — Use typed fields in place of `serde_json::Value`. Where the reference
  forwards a value untouched, keep its raw JSON or bytes in a named newtype so
  the response bytes and ETag are preserved.
- **T7** — Follow the Rust API Guidelines. Derive common traits where they
  apply, use the `as_`, `to_`, and `into_` naming conventions, and keep enums
  used within the workspace exhaustive.
- **T8** — Group fields that occur together in a struct, with `Option` around
  the group if it can be absent. Use an enum for mutually exclusive cases.
- **T9** — Add a trait for a capability shared by multiple implementations and
  use an associated type when each implementation has one choice of type.
  Pass a value when the caller supplies it once per call.

## Documentation rules

Write the documentation with the types and signatures, following the
[documentation rules](review/rubric.md#d-documentation).

- **D1** — Document every public item, including fields, variants, and modules.
  Say what each represents.
- **D2** — State the rules on values, including units and ranges. For upstream
  types, link to the reference schema and identify which request returns the
  data.
- **D3** — Use `# Errors` to say when each error is returned. Document any
  remaining panic conditions in `# Panics`.
- **D4** — Explain which code calls a module and which code it calls.

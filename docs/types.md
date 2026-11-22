# Types for a route

Use the reference's schema, conversion, and fixtures to work out which input
the route accepts and which response the apps receive. Preserve both under
[rule P0](review/rubric.md#p-matching-the-reference).

## Where the types go

- **App responses** live in one crate, [`ccc-schema`](../ccc-schema), with one
  module for each API area, such as `food`, `news`, or `jobs`. A conversion
  imports its own area's module only. Types used by more than one area live
  in a shared module. Both builds use this crate, which depends on no upstream
  library, runtime, or web framework and builds for `wasm32-unknown-unknown`.
- **Upstream responses, conversions, and conversion errors** live together
  in a shared library. Put each site's schema and conversion code in that
  site's module or its submodules. This library builds for
  `wasm32-unknown-unknown`, performs no fetching, and depends on no runtime,
  HTTP client, or web framework. It depends on `ccc-schema` to construct app
  response values. The code that fetches upstream data is specific to each
  build. Conversions do no I/O and take any date or time they need as input.

## Review rules

Apply the [type rules (T1–T9)](review/rubric.md#t-types) in the module that
owns each type or conversion. Review the upstream and app response types on
their respective sides of the conversion (T1) and the conversion signature
and any error types in the upstream library (T2).

Apply the [documentation rules (D1–D4)](review/rubric.md#d-documentation)
where the item is defined. Keep the reference schema link with the upstream
type and error documentation with the conversion.

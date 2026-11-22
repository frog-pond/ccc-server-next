---
name: port-route
description: Port one route or one upstream source from the reference ccc-server into this repository. Use when asked to port, implement or reimplement a CCC route, source or upstream conversion, or to work an issue for a row of docs/reference/routes.toml.
---

# Bring one route over from the reference

One conversion per branch and pull request: one shape function in the reference and its source, with every entry of `docs/reference/routes.toml` that uses it (often the same path for both colleges). An issue for an upstream may hold several conversions; each is its own pull request. The pull request starts as a draft holding the types and signatures, and the bodies are added to the same pull request once the types are approved. The reference decides what is correct. `AGENTS.md` and `docs/review/rubric.md` decide how it is written here.

This is the part of `work-issue` that is specific to a route: branching, the pull request and handing over are described there. Follow `design-types` and use [the type layout](../../../docs/types.md) to place schemas, conversions and errors.

This procedure was written before any route had been brought over. Steps marked *provisional* describe the intent, and should be replaced with what was actually done once the first few routes are approved.

## 1. Understand the route before writing code

1. Find the entries by their `id`. What to do depends on `kind` (the legend is at the top of `routes.toml`). If an entry's `kind` is `board`, stop and say so: those are designed in #435. If it is `ops`, port it only if the issue says to.
2. Run `tools/reference-path` and read, in the reference, every file the entry names: where the route is registered, how its data is fetched, the functions that convert it, and their tests and fixtures. Read the tests. They state what must happen more exactly than the README does.
3. Read "How the server is built" in `AGENTS.md`, and the parts of `docs/reference/cache-semantics.md` and `docs/reference/upstreams.md` about this route's data.
4. Write a summary in the issue or the pull request description, not in a file:
   - what the outside site sends, and which fields can really be absent, with the fixture or schema line that shows it;
   - what the apps receive;
   - every failure: status, body, `Cache-Control`;
   - the resource's name, its parameters and how long its value stays fresh;
   - anything in the reference you could not explain.

## 2. Types and signatures first

5. Write the types and signatures only, with `todo!()` bodies:
   - types for what the site sends and what the apps receive, starting from the reference's schema and following [T1](../../../docs/review/rubric.md#t-types);
   - the conversion signature and any error types, following [T2](../../../docs/review/rubric.md#t-types);
   - the description of the resource: its name, parameters and how long its value stays fresh.

   Write the documentation with the types, not afterwards: every public type, field, variant and function says what it represents, any rule about its contents, and for a site's type where its shape comes from in the reference. The reviewer reads the documentation to judge the types. For each type, ask whether a struct or an enum fits the data, and whether an `Option` or a `String` could be something more exact.
6. Push them, open the draft pull request, and get them reviewed before writing the bodies. The first route that reads from a given upstream needs the more thorough review (`arch-review` skill) and the maintainer's approval of the types. A later route for the same upstream that adds no new public types writes its bodies without waiting, and says so in its pull request.

`todo!()` is allowed only while the pull request is a draft. A draft is never merged.

## 3. Write the bodies against the reference's outputs

7. Expected outputs come from the reference, never from your reading of what looks right. Take each input and its expected output from the reference's fixtures and tests, and say which test you took it from. *Provisional:* a script that runs the reference's conversion functions over its fixtures is planned and does not exist yet; until it does, copy the pairs from the reference's test files by hand.
8. *Provisional:* keep these input and output pairs beside the crate that owns the conversion, the way the nearest approved route does. If no route has been approved yet, ask where they should go.
9. Conversions do no I/O and read no clock or environment. That is what lets both builds share them.
10. Tests never use the network. If you think a fixture is missing, ask the maintainer. Do not fetch from a live site.

## 4. Finish

11. `mise run check` passes.
12. For any crate meant to be shared by both builds: `cargo check --target wasm32-unknown-unknown -p <crate>` passes.
13. Run the `arch-review` skill and deal with what it finds.
14. In the pull request description: the summary from step 4, the reference commit from `docs/reference/REFERENCE_COMMIT`, and anything that behaves differently from the reference. There should be nothing. A deliberate difference is agreed in an issue first.
15. If you found `docs/reference/` wrong or incomplete, fix it in the same pull request.

## Do not

- Copy how the old crates are written (`ccc-types`, `ccc-upstream-handlers`, `ccc-upstream-proxy`, `ccc-server-routes`).
- Loosen a type to make a test pass. If a fixture does not fit your type, either the type is wrong or the fixture shows a case the error enum should name.
- Bring over two conversions in one pull request because they looked alike.

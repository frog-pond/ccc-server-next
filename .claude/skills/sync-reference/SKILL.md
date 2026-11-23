---
name: sync-reference
description: Bring this repository's extracts of the reference ccc-server up to date after the reference has changed, and work out which ported or planned routes are affected. Use when asked to sync, update or re-pin the reference, or when tools/reference-drift reports drift.
---

# Sync with the reference

The reference keeps moving. `docs/reference/` describes it at the commit in `docs/reference/REFERENCE_COMMIT`. This skill moves that pin forward without losing track of what changed.

## Steps

1. Run `tools/reference-drift`.
   - If it reports local modifications in the reference checkout, stop and tell the user. Do not clean them up yourself.
   - If it reports that the recorded commit is not an ancestor of the reference's head, stop and tell the user.
   - If the reference is at the pinned commit, there is nothing to do.
2. If the user wants the latest reference, they update that checkout. Do not run `git pull` or `git checkout` in it yourself.
3. Read the whole difference between the recorded commit and the reference's head (`git -C "$(tools/reference-path)" diff <recorded>..HEAD`), using the script's lists of commits and files as a guide. The script lists every changed file, not only code. Sort each change:
   - **Route table change**: a route added, removed or re-pointed. Update `routes.toml`.
   - **Shape change**: a conversion's output changed. Note which rows name that shape function.
   - **Cache behaviour change**: anything under the source cache. Update `cache-semantics.md`, and flag it: the freshness crate's tests must change too.
   - **Upstream change**: a host, request or quirk. Update `upstreams.md`.
   - **Configuration, tests and fixtures**: `wrangler.jsonc`, a renamed fixture, a changed test. Update whatever in the extracts cites them.
   - **No effect on the contract**: refactors, tooling. Say so in one line.
4. Update the extracts with `file:line` citations at the new commit. Re-check every citation into a reference file that changed, whether or not the text around the citation changed: line numbers move.
5. Re-copy changed fixtures from the new commit as [the fixture README](../../../fixtures/ccc-server/README.md) describes. Update paths in the generator, fixture README and affected Rust tests if the reference moves a file.
6. Write the new commit to `docs/reference/REFERENCE_COMMIT`, then re-run each case in `tools/reference-fixture.mjs` with `tools/reference-fixture <case>`. Review the expected-output diffs and commit the regenerated files under `fixtures/ccc-server/generated/` with the pin.
7. Report to the user:
   - routes added or removed;
   - already-ported routes whose reference behaviour changed, each needing a follow-up;
   - changes to cache behaviour;
   - anything you could not classify.
8. Propose issues for the follow-ups, and create them only when the user agrees.

## Keep in mind

- One pull request for the re-pin and extract updates. Porting the changes is separate work.
- Do not record porting status in the extracts.
- If the drift is large, do the route table first: it tells you which other changes matter.

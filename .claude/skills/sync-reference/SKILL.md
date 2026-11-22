---
name: sync-reference
description: Bring this repository's extracts of the reference ccc-server up to date after the reference has changed, and work out which ported or planned routes are affected. Use when asked to sync, update or re-pin the reference, or when tools/reference-drift reports drift.
---

# Sync with the reference

The reference keeps moving. `docs/reference/` describes it at the commit in `docs/reference/REFERENCE_COMMIT`. This skill moves that pin forward without losing track of what changed.

## Steps

1. Run `tools/reference-drift`.
   - If it reports local modifications in the reference checkout, stop and tell the user. Do not clean them up yourself.
   - If the reference is at the pinned commit, there is nothing to do.
2. If the user wants the latest reference, they update that checkout. Do not run `git pull` or `git checkout` in it yourself.
3. For each commit the script lists, read its diff in the reference (`git -C "$(tools/reference-path)" show <sha>`) and sort it:
   - **Route table change**: a route added, removed or re-pointed. Update `routes.toml`.
   - **Shape change**: a conversion's output changed. Note which rows name that shape function.
   - **Cache behaviour change**: anything under the source cache. Update `cache-semantics.md`, and flag it: the freshness crate's tests must change too.
   - **Upstream change**: a host, request or quirk. Update `upstreams.md`.
   - **No effect on the contract**: refactors, tooling. Say so in one line.
4. Update the extracts with `file:line` citations at the new commit. Re-check the citations in any section you touched: line numbers move.
5. Write the new commit to `docs/reference/REFERENCE_COMMIT`.
6. Report to the user:
   - routes added or removed;
   - already-ported routes whose reference behaviour changed, each needing a follow-up;
   - changes to cache behaviour;
   - anything you could not classify.
7. Propose issues for the follow-ups, and create them only when the user agrees.

## Keep in mind

- One pull request for the re-pin and extract updates. Porting the changes is separate work.
- Do not record porting status in the extracts.
- If the drift is large, do the route table first: it tells you which other changes matter.

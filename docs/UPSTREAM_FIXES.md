# Upstream fixes

Fixes RustOut carries for its dependencies until they are released upstream.
Each one is a patch applied to an otherwise unchanged copy of the published
crate.

## miniquad 0.4.11: deleted textures stay in the binding cache

| | |
| --- | --- |
| Status | Fix proposed upstream, not merged or released yet |
| Issue | [not-fl3/macroquad#1006](https://github.com/not-fl3/macroquad/issues/1006) – "Text rendering corrupted - rendering as black rects" |
| Reproduction | [ricande/miniquad-atlas-repro](https://github.com/ricande/miniquad-atlas-repro) |
| Pull request | [not-fl3/miniquad#691](https://github.com/not-fl3/miniquad/pull/691) |
| Patch | [`vendor/patches/miniquad-0.4.11-forget-deleted-texture.patch`](../vendor/patches/miniquad-0.4.11-forget-deleted-texture.patch) |
| Vendored copy | `vendor/miniquad-0.4.11/` = crates.io miniquad 0.4.11 (`.crate` SHA-256 `dcbaea97…1f59e65`, git commit [4f13d4a](https://github.com/not-fl3/miniquad/commit/4f13d4a70caf6a8cbf0187a95461658e7812b566)) + the patch, used through `[patch.crates-io]` in `Cargo.toml` |

**Symptom.** From some point in a run, all text is drawn as black
rectangles, in the window and in recorded videos, until the game restarts.

**Cause.** When macroquad's font atlas is full, it deletes its texture and
creates a bigger one. miniquad's `delete_texture` leaves the deleted texture
in its texture-binding cache. If the new texture gets the same GL name and
the cache still records the old one as bound, the new texture is never bound
for its upload and stays empty, so it samples as opaque black. It happens
when the atlas grows right after a flush that ended with text, such as the
switch back from RustOut's video render target.

**Patch.** After `glDeleteTextures`, `delete_texture` calls a new
`GlCache::forget_texture`, which resets every cache entry that records the
deleted texture as bound (and the stored binding) to 0. No GL calls.

**Verification.** Found and verified in
[RustyHusky](https://github.com/ricande/RustyHusky/blob/main/docs/UPSTREAM_FIXES.md),
which grew out of RustOut and shares its drawing and recording: with the
reproduction on Linux/Mesa/Intel, black text in 6 of 10 runs unpatched and
0 of 10 (plus 0 of 12 instrumented runs) with the patch. RustOut itself was
checked with its UI tour; the failure was not reproduced in RustOut
specifically, so this is a fix for a shared, demonstrated cause, not for an
observed RustOut run. `scripts/check-vendored-miniquad.sh` (also in CI)
checks that the copy is exactly the crate plus the patch.

**Removing the copy.** Once a miniquad release contains the fix: check it
with the reproduction, delete `[patch.crates-io]` from `Cargo.toml`,
`vendor/`, `scripts/check-vendored-miniquad.sh` and its CI step, run
`cargo update -p miniquad`, the checks in AGENTS.md and the UI tour, and note
it in the CHANGELOG.

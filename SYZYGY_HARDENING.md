# Syzygy hardening branch

Branch: `syzygy/hardened-v0.12.0`

Upstream baseline: `jrmuizel/pdf-extract` tag `v0.12.0`, commit `b95bf9f`.

This branch carries malformed-input and resource-exhaustion fixes from
`https://gitlab.com/cxxl/pdf-extract`, preserving the original commit authors.
It also pins the hardened parser revisions consumed by those fixes:

- `adobe-cmap-parser` at `70b000d88f4f3b86010c248dd5f8f236f1555e61`
- `cff-parser` at `fb7a4b57fa8fc6800623776cdfe28c4d9cee8374`
- `type1-encoding-parser` at `401328a52586a6a220205e290e04fd1bfc7ae5ba`

The production `src/` tree was audited for direct `panic!`, `unwrap`, `expect`,
and assertion calls. The pinned parser revisions were audited with the same
scope; their remaining unwrap/assert calls are test-only, while CFF production
code retains debug-only invariant assertions guarded by input validation.

The unused `postscript` dependency was removed. `pdf-extract` never referenced
it, and its production parser contains unimplemented operators that should not
remain in the desktop dependency graph without a call site or regression test.
The `lopdf` extraction path was also reviewed: remaining explicit panic sites
are test-only, outside text extraction, or guarded by parser/internal-state
invariants; no second malformed-input abort equivalent to the Type3 failure was
identified.

Validation:

The upstream release is still 0.12.0 as of the 2026-09-08 dependency audit:
<https://github.com/jrmuizel/pdf-extract/releases/tag/v0.12.0>.
The malformed-input fixes and pinned parsers therefore remain necessary.
The blanket `allow(warnings)` has been removed. Disabled diagnostic bindings
are explicitly marked unused and lifetime signatures are explicit. Only the
upstream's unevaluated color/function metadata retains documented, item-level
`expect(dead_code)` annotations; new compiler warnings remain visible.

```text
cargo check --lib
cargo test --lib
cargo test -p syzygy-search --all-features
```

The upstream integration suite downloads linked PDF fixtures at test time. It
must be run in an environment that can reach those fixture hosts; a network
failure is not a production parser failure. Syzygy keeps its malformed Type3
regression fixture in the consumer test suite so that the original desktop
crash remains reproducible offline.

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

Validation:

```text
cargo check --lib
cargo test --lib
```

The upstream integration suite downloads linked PDF fixtures at test time. It
must be run in an environment that can reach those fixture hosts; a network
failure is not a production parser failure. Syzygy keeps its malformed Type3
regression fixture in the consumer test suite so that the original desktop
crash remains reproducible offline.

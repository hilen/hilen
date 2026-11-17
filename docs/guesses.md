# Unproved guesses

Proposed changes that sounded right but had no evidence behind them. They live here
instead of in the code. A guess graduates out of this file only with proof: an A/B per
[benchmark.md](benchmark.md) acceptance criteria for performance claims, or a reproduced
real-world failure for correctness claims. A synthetic test that panics on code nobody
writes is not a reproduced failure.

## Parked

### UIEvent dead subscriber pruning

Claim: a view that dies while subscribed to a `UIEvent` crashes the app on the next
trigger, so `trigger` should skip dead subscribers.

Evidence found: panic is reproducible only synthetically. No live crash path exists -
`TextField` hand-guards with `is_null` and unsubscribes on deselect, buttons subscribe
to their own events and die with them, the root view never dies. Liveness checks cost
5-20 ns per trigger; the "faster" rewrite attempted alongside measured 2.6x SLOWER
(measured in the old standalone `ui` crate, its `ui_event` bench did not move into this repo).

Verdict: parked. Revisit if a real crash from a dead subscriber ever shows up.

### Keep a page drawing while a screenshot waits

Claim: on WebGL the copy of a frame reaches a screenshot only when the device is polled,
so a still page that draws no frame never answers a screenshot. The fix kept the page
drawing and polling while a copy waited, a counter of waiting copies in
`window/state.rs`, and the UI test `Shot of still page`, on the branch
`still-page-screenshot`.

Evidence found: a screenshot of a still screen on an LG C1 on 2026-10-21 never came
back. The test passed without the fix in Firefox on a Mac. On 2026-11-16 the CI browsers
hung in `Inspect frames`, and the fix did not end it, the cause there was `after` on a
worker, see `docs/dispatch.md`.

Verdict: parked. Revisit with a reproduced still page screenshot that hangs.

## Retracted claims

Kept as a reminder of what happens without proof:

- "index walk is +95% in debug" - compared two different harness versions. Real A/B
  said +7.7%.
- "removing per-frame allocations made it slower" - measured under Docker, browser and
  Spotlight load. Clean A/B said +3.6-7.7%.
- "leaving a screen with a text field crashes on the next keypress" - never reproduced,
  the guard in `text_field/editing.rs` already prevents it. Found by reading the code after
  making the claim, which is the wrong order.

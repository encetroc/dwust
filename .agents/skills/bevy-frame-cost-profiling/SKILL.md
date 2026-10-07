---
name: "bevy-frame-cost-profiling"
description: "Diagnose Bevy FPS regressions by separating end-to-end frame latency from CPU update costs."
version: 1
created: "2026-10-01"
updated: "2026-10-01"
---
## When to Use
When FPS drops persist despite fast subsystem benchmarks, or when comparing rendering settings in a Bevy app.

## Procedure
1. Use the application's full startup scene/plugins and a fixed camera/resolution. Run a windowless image-target benchmark using the existing headless-verification workflow; inject actual camera rays so input costs are covered.
2. After pipeline/asset warm-up, collect at least 40 wall-frame intervals and separately measure First-to-Last update duration. Log adapter/backend, scene counts and median/p95. Serialize benchmark runs to avoid shared GPU contention.
3. Count Changed<Transform>/Changed<Projection> on stationary cameras and Modified mesh/material asset events. Assert idle writes are zero; ensure the same monitor is red-capable before optimizing.
4. Change one variable per A/B run: AA mode, shadow configuration, shader work or gameplay system gating. For orthographic scenes compare default 4x MSAA against Msaa::Off plus Fxaa::default(), using the matching-version official anti_aliasing example.
5. Keep only measured wins. Preserve geometry, collision and shadows unless independently validated. Use set_if_neq and read-before-mutable-dereference guards to stop camera dirtiness; check asset values through immutable Assets::get before calling get_mut.
6. Run scripted camera/editor/grab/build journeys, inspect fresh A/B captures, and document a reproducible comparison command. Re-run on the user's target adapter for hardware FPS claims.

## Pitfalls
- llvmpipe/software Vulkan provides comparative evidence, not hardware FPS guarantees.
- Fast reservation/support-picking or First-to-Last timings alone do not establish FPS; GPU rendering/presentation may dominate.
- MeshPickingSettings APIs existing in dependencies do not mean MeshPickingPlugin is installed. Inspect actual plugin composition before blaming automatic picking.
- Changing AA can soften small details. Inspect fine seams, station guides, HUD text and silhouettes before accepting the tradeoff.

## Verification
1. Stable scene counts and identical camera/resolution across comparisons.
2. Reported median/p95 wall latency improves; idle camera/asset assertions pass.
3. Real input journeys and inspected screenshots preserve gameplay and visual landmarks.
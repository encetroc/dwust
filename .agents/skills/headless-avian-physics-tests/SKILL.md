---
name: "headless-avian-physics-tests"
description: "Verify Avian 3D rigid-body behavior in Bevy without creating a window"
version: 1
created: "2026-09-22"
updated: "2026-09-22"
---
## When to Use
Use when testing Bevy 0.19 Avian 3D gravity and collision behavior in unit tests or other windowless verification.

## Procedure
1. Build the test App with MinimalPlugins, AssetPlugin::default(), MeshPlugin, PhysicsPlugins::default(), and the feature plugin under test.
2. Insert TimeUpdateStrategy::ManualDuration with a fixed timestep, call app.finish() before the first update, and use app.update() to run deterministic simulation steps.
3. Query Avian Position and velocity components after enough fixed updates; assert physical outcomes such as settling height and containment against static colliders.
4. Use direct ECS component changes for controlled test setup only, updating both Position and Transform when relocating a body before simulation.
5. Never invoke cargo run or create a Window for physics-only verification.

## Pitfalls
- MinimalPlugins plus PhysicsPlugins can panic when Avian systems run before plugin finish; call app.finish() first.
- Avian asset/collider systems require AssetPlugin and MeshPlugin resources even when the test does not render.
- Relocating only Position can leave the collider/transform state stale; update Transform together for controlled repositioning.

## Verification
1. cargo fmt --check passes.
2. cargo check and cargo clippy --all-targets -- -D warnings pass.
3. cargo test passes with deterministic gravity and wall-containment assertions.
---
name: "bevy-headless-verification"
description: "Use when developing or testing a Bevy game headlessly without opening a window, including scripted gameplay assertions, off-screen screenshots, visual inspection, or CI verification."
version: 9
created: "2026-09-17"
updated: "2026-10-01"
---
## When to Use
Use for real render-headless Bevy playtests and visual checks: the game must run with rendering enabled, receive scripted inputs, produce off-screen PNG checkpoints, and have those images inspected. Use simulation-only tests separately when no rendered evidence is needed.

## Procedure
1. Resolve the game's Bevy version from Cargo.toml/Cargo.lock and read references/official-bevy.md for version-specific APIs. Match tagged Bevy sources to the resolved version; the bundled smoke harness targets Bevy 0.19.1 and is infrastructure evidence only.
2. Inspect the repository for existing verifier binaries, journey/scenario tests, artifact conventions, and normal test/lint commands. Reuse them when they exist. Otherwise define a verification-only entry point or plugin that preserves normal launch behavior. Keep project-specific gameplay and input seams in the game repository, not in a replacement demo.
3. Define a deterministic scenario before coding: starting state, seed, press/hold/release actions and timing, observable assertions, and named before/action/after screenshot checkpoints. Use the game's real plugins, assets, camera, HUD, action handlers, and gameplay systems. Direct ECS mutation or a replacement scene proves only a fixture.
4. Configure true windowless rendering. Retain DefaultPlugins, RenderPlugin, the game's render stack and asset systems; set WindowPlugin.primary_window = None and ExitCondition::DontExit, disable WinitPlugin, and use ScheduleRunnerPlugin or the official externally-driven lifecycle. Assert that game plugins do not spawn Window entities. An invisible window, Xvfb, MinimalPlugins, or a disabled renderer is a different workflow.
5. Render the actual game camera to a fixed-size Image target and retain its handle. In Bevy 0.19.1 use Image::new_target_texture and RenderTarget::Image; adapt constructors and conversions for other versions. Preserve camera layers, postprocessing and HUD, explicitly attach UI with UiTargetCamera when required, and provide a verification seam for code that assumes a primary Window or OS cursor.
6. Make time and startup repeatable. Seed project RNGs, use manual time/fixed-step control where supported, record simulation ticks, wait for asset/state readiness, then perform bounded render warm-up. Keep a frame watchdog and outer wall-clock timeout. Pipeline compilation does not replace asset readiness.
7. Drive the input layer actually consumed by the game. For ButtonInput inject press/release after InputSystems and before gameplay consumers; keep held buttons pressed across frames. Inject raw input messages before InputSystems when testing raw subscribers. For action abstractions use their supported seam and state the coverage boundary. For UI/picking follow the versioned render-to-texture pointer approach with move/press/release coordinates; do not set Interaction directly.
8. At each checkpoint, assert state after the relevant gameplay systems, request Screenshot::image(target_handle.clone()), and acknowledge completion only after ScreenshotCaptured is converted and saved successfully. Hold the checkpoint state steady while asynchronous capture completes. Track requested/completed captures, use unique output directories, remove stale files, and allow at most one in-flight request per target.
9. Build before applying the runtime timeout. On Linux run the built executable with DISPLAY and WAYLAND_DISPLAY unset, from the correct asset root, and record the actual graphics adapter/backend. A compatible GPU or software adapter is required; no adapter is an environment block, not a passing headless test. Keep renderer, asset and capture errors visible.
10. Run the repository's unit/integration journeys, formatter and linter as applicable, then run the render-headless verifier. If no project verifier exists, run the bundled smoke harness from this skill directory to separate environment failures from game failures; its success does not verify the user's game.
11. Verify every required PNG is fresh, decodable and the expected size; inspect the actual images with the image-reading tool. Compare before/after landmarks, movement, camera framing, occlusion, lighting, animation and HUD against the scenario. A nonempty file, pixel difference or passing simulation test alone is insufficient.
12. Report scenario/actions, assertion results, screenshot paths, adapter/backend, Bevy version, resolution, final state, capture count, commands, and unsupported coverage. Mark verification passed only when real gameplay, checked saves, assertions and visual inspection agree; distinguish simulation-only success, smoke-harness success, missing evidence and environment-blocked runs.
## Pitfalls
- Bevy APIs, render-target conversions, input system sets, screenshot events and lifecycle pumping vary by version; match the target release.
- Bevy 0.19.1 synthetic MouseWheel messages require phase: bevy::input::touch::TouchPhase::Moved as well as unit, x, y, and window. Confirm the game consumer's use of window IDs.
- Screenshot readback is asynchronous. Pump until checked ScreenshotCaptured saves complete; sleeps, extra frames, and Assets<Image>.data do not prove capture success. Gate the next input stage on completed captures.
- For transient flicker, capture the action frame and consecutive successor frames before any settling delay. Consecutive-frame capture may have multiple readbacks in flight: bind each observer to its stage/frame, track all completions, and keep the stage unchanged until all captures arrive.
- Use localized object-color masks that include semi-transparent appearance and exclude player/terrain/HUD. Inspect a failing image before concluding a zero mask means disappearance; intentional occlusion needs a separate expectation.
- Match synchronous_pipeline_compilation to the real application. Enabling it only in the harness hides first-use asynchronous pipeline flicker.
- Screenshot::primary_window() has no target in a windowless run. Capture the image target and check save/conversion errors.
- ButtonInput edges are cleared by InputSystems; inject afterwards. One render update is not necessarily one fixed tick.
- Drive the real input consumers and preserve the game camera/scene; direct outcome mutation and setting Interaction do not establish gameplay or UI coverage.
- Bevy 0.19.1 retained Gizmo extraction ignores Visibility. Hide static overlays by detaching Gizmo while retaining its descriptor/asset handle, and reinsert to show. Check rendering components and on/off screenshots.
- Exact pixel hashes vary across adapters, drivers, fonts and settings. Prefer tolerances plus visual inspection. Inspect HUD glyphs and contrast at both zoom extremes.
- Use isolated writable artifact directories and remove stale captures. Disclose missing audio and investigate rendering/asset errors.
- Multiple renderer instances in one process may require LogPlugin isolation.
## Verification
1. No visible or hidden Window entity is created; executable succeeds with display variables unset on Linux, or platform-equivalent no-window configuration is documented.
2. Scripted inputs exercise the real game consumers, including edge, held and release behavior; gameplay assertions pass.
3. Every checkpoint has a fresh, checked, decodable PNG of expected dimensions before exit; missing captures, failed assertions and watchdog expiry fail the run.
4. Open relevant screenshots and describe visible evidence, distinguishing gameplay coverage from infrastructure smoke tests.
5. Run repository tests, formatter/linter and verifier; report commands, artifact paths, Bevy version/adapter and unsupported coverage.
6. For focus orbit cameras, exercise entry → orbit → exit → re-entry with rays projected from the actual camera. Assert intended viewport position, orbit radius/elevation, intermediate tween values and restored zoom. Changed-angle re-entry detects occlusion that near-target rays bypass.
7. For intentionally off-center orthographic framing, assert normalized viewport coordinates at multiple zoom levels and orbit angles. To measure physical orbit radius/elevation, subtract the camera-up screen offset (view height × framing fraction × transition blend) from camera translation first.

## Shadow diagnostics
1. Capture the stationary real scene with shadow maps enabled and disabled, waiting for render/readback completion after the toggle. Count pixels darkened by shadows in isolated object regions; verify the pre-fix capture fails the same assertion.
2. Project each mesh's local Aabb center through GlobalTransform and Camera::world_to_viewport. Transform translation alone is incorrect for world-space-authored meshes at identity transforms.
3. Tune directional normal bias independently from depth bias: normal bias is measured in shadow texels, depth bias in world units. Small objects can disappear under the default normal offset; reducing depth bias can instead introduce terrain acne. Inspect both close and wide screenshots.
# Official Bevy sources and implementation notes

## Version boundary

Reference version: **Bevy 0.19.1**. The packaged source/examples in the Cargo registry were inspected, and the tagged official headless examples and versioned rustdoc were fetched. Resolve the user's actual dependency version before implementing. Use `https://github.com/bevyengine/bevy/blob/v<VERSION>/...` and `https://docs.rs/bevy/<VERSION>/...`; `main` and `latest` may describe incompatible APIs. A Cargo registry copy of the exact version is also an authoritative local source.

The verification workflow and assertions are recommendations synthesized from these APIs, not an official Bevy playtesting framework. The bundled harness validates the windowless rendering/input/capture mechanism; it does not play or verify an arbitrary user's game.

## Primary sources

| Official source | What it establishes |
| --- | --- |
| [Externally driven headless renderer](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/app/externally_driven_headless_renderer.rs) | WindowPlugin with no primary window and DontExit; Winit disabled; RenderPlugin synchronous pipeline compilation; image render target; manual finish/cleanup and SubApps updates; device polling; Screenshot::image. Best reference for an externally controlled runner. |
| [Headless renderer](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/app/headless_renderer.rs) | ScheduleRunnerPlugin instead of a window event loop; camera-to-image rendering; warm-up; low-level texture-to-buffer readback, padding, mapping and polling. Its header recommends Screenshot for individual captures. |
| [Screenshot example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/window/screenshot.rs) | Spawn a screenshot entity and attach a save observer. This example captures a window; replace primary_window with image for the headless workflow. |
| [Screenshot rustdoc](https://docs.rs/bevy/0.19.1/bevy/render/view/window/screenshot/struct.Screenshot.html) and [implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_render/src/view/window/screenshot.rs) | Capture is asynchronous; ScreenshotCaptured is the completion event; entity despawns after capture. Screenshot::image accepts Handle<Image>. save_to_disk logs errors rather than returning failure to the runner. Public example import is bevy::render::view::screenshot. |
| [TimeUpdateStrategy](https://docs.rs/bevy/0.19.1/bevy/time/enum.TimeUpdateStrategy.html) | ManualDuration advances simulated time by a specified duration per update. 0.19.1 also exposes FixedTimesteps; check availability before using on older releases. |
| [ButtonInput](https://docs.rs/bevy/0.19.1/bevy/input/struct.ButtonInput.html), [input plugin](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_input/src/lib.rs), [keyboard implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_input/src/keyboard.rs) | InputSystems runs in PreUpdate. keyboard_input_system clears transient ButtonInput edges then consumes KeyboardInput messages. Direct ButtonInput injection must run after that clearing and before the game's consumers. |
| [Keyboard example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/input/keyboard_input.rs) | Gameplay consumption of pressed, just_pressed and just_released. |
| [UI rendered to texture](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/ui/render_ui_to_texture.rs) | Image camera target; UiTargetCamera; custom pointer with image-target Location and PointerInput move/press/release. Useful building blocks for off-screen UI tests, although the example itself uses a window. |
| [Simulation-only headless example](https://github.com/bevyengine/bevy/blob/v0.19.1/examples/app/headless.rs) | A distinct mode that can omit graphics/window features; not suitable for screenshot verification. |

## Smallest practical integration (0.19.1)

Use the game's real app builder; these are configuration fragments, not a standalone game:

```rust
use bevy::{
    app::ScheduleRunnerPlugin,
    camera::RenderTarget,
    prelude::*,
    render::{render_resource::TextureFormat, RenderPlugin},
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use std::time::Duration;

app.add_plugins(
    DefaultPlugins
        .set(WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        })
        .set(RenderPlugin {
            synchronous_pipeline_compilation: true,
            ..default()
        })
        .disable::<WinitPlugin>(),
)
.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0)))
.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));

// In a startup/setup system; insert on the actual game camera.
let target = images.add(Image::new_target_texture(
    1280, 720, TextureFormat::Rgba8UnormSrgb, None,
));
commands.entity(game_camera).insert(RenderTarget::Image(target.clone().into()));
```

`ScheduleRunnerPlugin::run_loop` controls pacing; it does not make simulation time deterministic by itself. The manual time strategy supplies simulation delta. Keep fixed-update input consumption and first-frame initialization in mind.

For a manual loop, follow the tagged externally-driven example's lifecycle. Plugin readiness, `finish`, `cleanup`, main/render updates and GPU progress still matter. Poll completion alone is not evidence that the main-world screenshot observer has run or a file was saved. Keep pumping until explicit capture acknowledgments or a deadline.

For ordinary snapshots, prefer Screenshot over writing a custom readback subsystem. The low-level headless_renderer example adds COPY_SRC for its direct texture copy and handles aligned rows; those implementation details should not be copied indiscriminately into a different capture path. Merely reading Assets<Image>.data does not fetch newly rendered pixels.

## Gameplay and UI input branches

### Resource-based keyboard input

Register scripted injection as `PreUpdate, script.after(InputSystems)` for games consuming ButtonInput later in the frame. If consumers are also in PreUpdate, explicitly order injection before their system set. Press once, hold across ticks, release once, and assert that all relevant edges were consumed. Do not clear held inputs every frame. Raw KeyboardInput messages instead need to arrive before InputSystems; resource injection does not test raw message subscribers or OS key translation.

### Action abstractions and pointer input

If the project reads an action resource/plugin rather than ButtonInput, inject through its supported input seam and state clearly which mapping layers are covered. Driving game commands can be a useful integration test but does not establish keyboard/pointer mapping correctness.

For picking/UI, use the official render_ui_to_texture example's custom pointer approach: a pointer identity, a `Location` whose normalized render target is the image, and `PointerInput` move/press/release messages. Adapt its pointer driver to scripted coordinates rather than OS window events, and schedule it in the version's picking input stage before hit testing and interaction consumers. Send move/press/release across appropriate ticks and let the actual picking/backend/UI handlers determine the hit and response. Confirm logical/physical coordinate mapping for target scale and viewport. ButtonInput<MouseButton> alone supplies no cursor position.

Attach HUD roots to the correct camera with UiTargetCamera. Game code requiring `Single<&Window>`, primary-window cursor coordinates, or window resize events needs a verification-compatible viewport/input provider. Directly assigning Interaction::Pressed skips picking and does not verify clicking. The bundled smoke test covers keyboard movement only; UI/picking requires a project-specific tested scenario.

## Checked capture and outcome protocol

Spawn `Screenshot::image(handle.clone())` with an `On<ScreenshotCaptured>` observer. Convert `event.image.clone().try_into_dynamic()`, save with a checked Result, and acknowledge completion only after success. Converting to RGB follows save_to_disk's choice to discard alpha (which can represent brightness with HDR). Create parent directories first.

For a checkpoint, record the assertion state and request tick; hold the relevant scene stable until capture completes. The callback tick may differ due to render/main-world latency. Track each checkpoint separately and avoid multiple simultaneous requests for the same target. Fail if any expected capture is missing. The official example's extra two updates illustrate latency, not a universal bound.

Asset readiness and shader readiness are separate. A fixed 40-frame warm-up works for the bundled procedural smoke scene, not necessarily a real game's asynchronous loading. Wait for the project's asset/state readiness, then bound rendering stabilization. Compare scene landmarks in the captured image to reject black/empty/incomplete frames.

## Run the bundled infrastructure smoke test

Resolve these paths relative to this skill directory, not the game repository. The harness is an isolated crate at `examples/smoke/` with an exact `bevy = "=0.19.1"` dependency. It uses a procedural 2D scene and no external assets. It is agent-authored, not a verbatim official example.

```bash
SKILL="$HOME/.pi/agent/pi-hermes-memory/skills/bevy-headless-verification"
BUILD_DIR=$(mktemp -d)
RUN_DIR=$(mktemp -d)
# May reuse a compatible existing Cargo target cache instead to reduce rebuild time.
CARGO_TARGET_DIR="$BUILD_DIR" cargo build --manifest-path "$SKILL/examples/smoke/Cargo.toml"
cd "$RUN_DIR"
env -u DISPLAY -u WAYLAND_DISPLAY timeout 120s \
  "$BUILD_DIR/debug/bevy-headless-skill-probe" >run.log 2>&1
# Require exit 0, PASS in run.log, and open both captures with the image-reading tool.
```

The harness asserts zero Window entities, press/hold/release consumption, movement from x=-60 to x=60, correct screenshot dimensions, visible green sprite pixels, successful PNG saves and completion before exit. Open captures/before.png and captures/after.png: the green square should move from the left to the right. The frame watchdog fails a stalled capture while the outer timeout covers a stalled render/device operation.

This recipe was compiled and executed on Linux with both display variables unset, using Mesa llvmpipe (CPU) / Vulkan. Both 320x240 PNGs were opened and the displacement confirmed. This is evidence for that configuration, not universal GPU/OS portability or verification of a user's game.

Headless rendering still needs a graphics adapter. Inspect actual adapter logs; software rendering can be much slower. If none is available, report the infrastructure blocker or use a compatible installed software Vulkan implementation. Avoid hard-coding this machine's ICD paths in portable scripts. Missing audio-device warnings do not invalidate a visual-only smoke test; they mean audio was not verified. Keep rendering/asset failures visible.

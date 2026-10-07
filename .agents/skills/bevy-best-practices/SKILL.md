---
name: "bevy-best-practices"
description: "Use for any Bevy Rust task involving ECS architecture, schedules, plugins, states, assets, rendering, UI, input, animation, shaders, platform targets, performance, or tests; consult the matching official example before implementing."
version: 1
created: "2026-09-19"
updated: "2026-09-19"
---
## When to Use

Use this skill for any Bevy application or library task. Treat the target repository's Bevy version and feature flags as authoritative: the official examples are versioned executable documentation, and APIs change quickly between releases.

## Procedure

1. **Pin the context.** Read the target `Cargo.toml` before choosing an API. Record the Bevy version, Rust edition, enabled features, and whether the task is main-world ECS, UI, 2D/3D rendering, asset processing, or render-world work. Then inspect that checkout's `examples/README.md`; it is the complete index and warns about release/development-version API differences. Completion criterion: every proposed type and feature belongs to the target version and feature set.
2. **Find the smallest canonical example.** Search `examples/` by category and read the complete closest example, not only its snippet. Prefer the simplest example that demonstrates the requested seam, then compare a showcase or stress example when architecture or scale matters. Completion criterion: the implementation choice is backed by a concrete example path and its neighboring asset/feature requirements are known.
3. **Shape a deep plugin.** Put one coherent feature in a plugin (or a small plugin group): its components, resources, messages/events, observers, systems, state wiring, and configuration should live together. Keep `main` responsible for composing plugins and selecting `DefaultPlugins`/`MinimalPlugins`. Completion criterion: the feature can be enabled, tested, and reasoned about without editing unrelated feature code.
4. **Model data for ECS.** Put per-entity behavior/state in small `Component`s, shared singleton/configuration in `Resource`s, and transient cross-system notifications in messages or triggered events. Add marker components for query intent and use `Name` where inspection/debugging benefits. Completion criterion: each piece of mutable state has one obvious owner and systems expose their access through precise parameters.
5. **Schedule deliberately.** Register systems with `.add_systems(Schedule, ...)`. Use the broadest parallelism that preserves correctness; add named `SystemSet`s with `.configure_sets`, or local `.chain()`, `.before()`, and `.after()` constraints only where data flow requires order. Use `run_if` for conditional work and put simulation in `FixedUpdate` when a fixed step is required. Completion criterion: every ordering dependency is explicit, and no correctness claim relies on incidental system order.
6. **Use the ECS boundary correctly.** Prefer `Commands` for deferred spawning, despawning, inserts, and removals; reserved entity IDs can be used immediately. Use `World`/exclusive systems only when immediate arbitrary world access is truly needed because they block parallel execution. Avoid holding conflicting mutable queries; split filters with `With`/`Without`, use `ParamSet`, `Single`, `Option`, `Result`, or a custom `QueryData`/`QueryFilter` where they state the invariant better. Completion criterion: the app schedules without unresolved ambiguities and the query signatures communicate cardinality and mutability.
7. **Verify behavior at the right boundary.** Run `cargo fmt --check`, `cargo check` or the repository's checks, focused tests, and the smallest relevant example. For visual or platform work, verify assets, feature flags, target backend, and a screenshot/headless path when possible. Completion criterion: compilation, focused tests, and the relevant runtime path all pass for the target version.

## Reference

### App composition and plugins

- Start with `App::new().add_plugins(DefaultPlugins)` for a normal interactive app. Customize a plugin with `.set(...)`, disable a plugin with `.disable::<...>()`, and add feature plugins in a tuple. `DefaultPlugins` is a `PluginGroup`, not a monolithic requirement.
- Make plugins narrow and configurable. `impl Plugin::build` should install the feature's resources, messages/events, observers, and systems; expose configuration as plugin fields/resources instead of hard-coding global behavior. Use `PluginGroup` only to compose related plugins. Canonical references: `examples/app/plugin.rs`, `plugin_group.rs`, `settings.rs`, and `showcase/game_menu.rs`.
- For command-line, server, or deterministic tests, use `MinimalPlugins` or `DefaultPlugins` with windowing/rendering disabled. Add only the input/window/time resources the test needs. For a manually driven loop use `set_runner`/`App::update`; for a windowless loop use `ScheduleRunnerPlugin`. References: `examples/app/headless.rs`, `custom_loop.rs`, `headless_renderer.rs`, and `tests/how_to_test_apps.rs`.

### ECS data and systems

- Components are small, composable data and tags attached to entities. Resources are shared process/world state. Systems are ordinary Rust functions whose parameters declare data access; keep them focused on one transformation and split setup, input, simulation, presentation, and cleanup.
- Spawn bundles as tuples of components. Reuse spawn logic with Rust constructors/builders and use `#[require(...)]` for component invariants that should be supplied automatically. Use `children![]`, `with_child`, and `with_children` for hierarchy; use `ChildOf`/`Children` and relationship APIs rather than manually maintaining reverse links. References: `showcase/breakout.rs`, `ecs/relationships.rs`, `ecs/hierarchy.rs`, and `ecs/component_hooks.rs`.
- `Commands` are deferred, but spawned IDs are reserved immediately. This makes command buffering the normal safe mutation path. Use an exclusive `fn(&mut World)` only for operations that cannot be expressed with system parameters; it serializes execution. Reference: `ecs/ecs_guide.rs`.
- Use filters to make queries exact: `With`, `Without`, `Added`, `Changed`, `Ref`, `Or`, `Has`, `Option`, and `RemovedComponents` each express a different lifecycle or cardinality fact. A `Single` parameter documents and enforces one matching entity; use fallible `Single`/`Option` patterns when startup or teardown can temporarily violate that invariant. For repeated complex queries, derive named `QueryData`/`QueryFilter`; mark a custom query `mutable` or `contiguous` only when it really needs those capabilities. References: `ecs/change_detection.rs`, `custom_query_param.rs`, `system_param.rs`, and `ecs_guide.rs`.
- Mutable access marks change detection on mutable dereference even if the value stays equal. Use `set_if_neq` for `PartialEq` data when downstream `Changed<T>` work should run only for a semantic change. Reference: `ecs/change_detection.rs`.
- Bevy systems run in parallel by default. Resolve real data-order dependencies with system sets and ordering constraints; inspect ambiguity warnings rather than silencing them. Use `.ambiguous_with(...)` only for a documented false positive. Reference: `ecs/nondeterministic_system_order.rs`.
- Run conditions are read-only systems returning `bool`; compose reusable conditions with `and_then`, `or_else`, `not`, `in_state`, resource checks, and input helpers. Conditions prevent work; they do not replace cleanup or state-transition schedules. Reference: `ecs/run_conditions.rs`.
- Use `Local<T>` for small system-owned state, not as a substitute for shared game state. Use a custom `SystemParam` when a repeated group of query/resource parameters represents a meaningful domain operation. References: `ecs/system_closure.rs`, `ecs/system_param.rs`, and `ecs_guide.rs`.

### Messages, triggered events, and observers

- In current Bevy examples, use `#[derive(Message)]`, `.add_message::<T>()`, `MessageWriter<T>`, `MessageReader<T>`, and `MessageMutator<T>` for queued per-frame communication. A message can be read by multiple systems; order writers/readers explicitly when same-frame processing matters. Use `MessageMutator` when one system must read and write the same message type. Reference: `ecs/message.rs`.
- Use `#[derive(Event)]` plus `commands.trigger(...)`/`On<T>` for immediate triggered behavior and observer-driven reactions. Entity-targeted events use `#[derive(EntityEvent)]`; lifecycle observers can watch `Add<T>` and `Remove<T>`. Attach local observers with `.observe(...)` and global ones with `.add_observer(...)`. References: `ecs/observers.rs`, `picking/simple_picking.rs`, and `ui/widgets/button.rs`.
- Choose the mechanism by delivery semantics: messages are buffered communication between scheduled systems; triggered events/observers are reactive callbacks attached to a trigger/entity. Do not assume a reader runs in the same order as a writer without schedule constraints.

### Schedules, state, time, and simulation

- Use `Startup` for one-time setup, `Update` for variable-rate presentation/game logic, `PreUpdate`/`PostUpdate` when integration with input or transforms requires a phase, and `OnEnter`/`OnExit` for state lifecycle. Add state with `init_state::<S>()`, transition with `NextState<S>`, and gate ongoing work with `run_if(in_state(...))`.
- Scope transient entities with `DespawnOnExit(S)` or a state marker plus cleanup. Use sub-states for nested screens and `ComputedStates` to derive orthogonal concepts such as `InGame`, `Paused`, or `TurboMode`; avoid multiplying every combination into one giant enum unless it is genuinely one state machine. References: `state/states.rs`, `state/computed_states.rs`, and `showcase/game_menu.rs`.
- Tick timers with the appropriate `Time` resource and `time.delta()`. Normal systems use virtual time by default; use `Time<Real>` for unscaled wall-clock behavior, `Time<Virtual>` for pause/speed control, and `Time<Fixed>` inside fixed-step systems. Reference: `time/virtual_time.rs` and `time/timers.rs`.
- For deterministic or collision-sensitive simulation, put the simulation in `FixedUpdate`, accumulate input before the fixed loop, and interpolate visual `Transform` values after it. Keep camera/input sampling at frame rate when responsiveness requires it. Reference: `movement/physics_in_fixed_timestep.rs`.

### Assets, scenes, glTF, and animation

- Asset loading is asynchronous and non-blocking. Load with `AssetServer`, retain typed `Handle<T>` values in components/resources, and check `Assets<T>` or `AssetServer` load/dependency state before using the asset. Use `GltfAssetLabel` for glTF subassets and `WorldAssetRoot` for spawning a loaded scene. References: `asset/asset_loading.rs`, `gltf/load_gltf.rs`, and `gltf/update_gltf_scene.rs`.
- Do not gate gameplay on one handle when dependencies matter: wait for recursive dependency loading or use an explicit loading state/barrier. Keep a loading screen alive until both assets and render pipelines are ready when first-frame presentation matters. Reference: `asset/multi_asset_sync.rs` and `showcase/loading_screen.rs`.
- Store shared meshes/materials as handles and clone handles across entities; create per-entity materials only when their values differ. Use `AssetServer` paths relative to the app's `assets/` directory. File hot-reloading needs the appropriate watcher/dev feature. References: `3d/pbr.rs`, `asset/hot_asset_reloading.rs`, and `asset/asset_loading.rs`.
- Define custom assets with `#[derive(Asset, TypePath)]`, register them with `init_asset`, and implement/register an `AssetLoader`; keep loader errors typed. For runtime-created data use `Assets<T>::add`. Reference: `asset/custom_asset_loader.rs`.
- For animation, wait until the glTF scene is instantiated, then configure `AnimationPlayer`/`AnimationTransitions` and attach the `AnimationGraphHandle` to the correct entity. Use animation graphs for blending and serialized graphs when the project needs authoring/reuse. References: `animation/animated_mesh_control.rs`, `animation/animation_graph.rs`, and `gltf/gltf_extension_animation_graph.rs`.

### Rendering and cameras

- 2D uses `Camera2d`, `Sprite`, `Mesh2d` plus `MeshMaterial2d<ColorMaterial>`, and `Text2d`; 3D uses `Camera3d`, `Mesh3d`, `MeshMaterial3d<StandardMaterial>`, lights, and transforms. Add the smallest camera/light/material setup that proves the feature. References: `2d/sprite.rs`, `2d/mesh2d.rs`, `3d/3d_scene.rs`, and `3d/pbr.rs`.
- Use `Transform` for world entities and `GlobalTransform` for propagated world-space results. Parent entities to express local transforms and let Bevy propagate hierarchy/visibility. Keep 2D scale's z component at `1.0` when z ordering matters; use camera order/render layers for multi-pass or split-screen composition. References: `3d/parenting.rs`, `3d/render_to_texture.rs`, `3d/split_screen.rs`, and `2d/rotation.rs`.
- Render-to-texture is an asset/image plus a camera `RenderTarget`, often combined with `RenderLayers` and camera ordering. Use it for mirrors, portals, minimaps, UI-in-world, or captures; do not introduce a custom render graph for this common case. Reference: `3d/render_to_texture.rs`.
- Custom materials use an `Asset + TypePath + AsBindGroup` type and `MaterialPlugin<M>`, with `ShaderRef` pointing at the asset shader. Start with `Material` defaults, then add only the specialized hooks needed. References: `shader/shader_material.rs`, `extended_material.rs`, and `shader_advanced/manual_material.rs`.
- Treat render-world APIs as a separate advanced boundary. Main-world data is extracted into `RenderApp` via `ExtractSchedule`, and render systems/graph nodes have explicit phase ordering and usually frame latency. Follow the extraction/headless-renderer examples before changing renderer internals. References: `ecs/extraction.rs`, `app/headless_renderer.rs`, `shader_advanced/custom_render_phase.rs`, and `shader_advanced/custom_post_processing.rs`.

### UI, input, picking, and windows

- Compose UI from `Node`, `Text`, `TextSpan`, `ImageNode`, style components, and `children![]`/scene builders. Layout is flex/grid/absolute positioning: make the parent define its size and layout, then use children for content. Use `px`, `percent`, and `Val` intentionally; nest text in a node when margins/padding belong to a container. References: `ui/layout/flex_layout.rs`, `ui/layout/grid.rs`, `ui/text/text.rs`, and `ui/images/image_node.rs`.
- The `bevy_ui_widgets` controls are behavior-only/headless. Add `Button`, `Checkbox`, `Slider`, `TabGroup`/`TabIndex`, etc., supply styling, and react to `Activate`/`ValueChange` or component changes with observers or change-filtered systems. Add `TabNavigationPlugin` for keyboard focus/navigation and `InteractionDisabled` for disabled controls. References: `ui/widgets/button.rs`, `ui/widgets/standard_widgets.rs`, `ui/widgets/standard_widgets_observers.rs`, and `ui/navigation/directional_navigation.rs`.
- Add `MeshPickingPlugin` or the relevant picking backend, then attach pointer observers to the entity being interacted with. Stop propagation when a handler owns the interaction. References: `picking/simple_picking.rs`, `picking/dragdrop_picking.rs`, and `picking/custom_hit_data.rs`.
- Read continuous input with `ButtonInput<T>::pressed`, edge input with `just_pressed`/`just_released`, and raw keyboard/text messages when the actual input stream matters. Use `KeyCode` for physical locations and `Key` for semantic characters; accumulate instantaneous actions when a fixed-step simulation can otherwise miss them. References: `input/keyboard_input.rs`, `input/keyboard_input_events.rs`, and `movement/physics_in_fixed_timestep.rs`.
- Configure windowing through `WindowPlugin`/`Window`, and use `Single<&mut Window>` or `Single<&mut CursorOptions>` when the singleton invariant is real. Gate platform-specific cursor/window features and handle WASM/mobile differences explicitly. Reference: `window/window_settings.rs`, `window/multiple_windows.rs`, and `mobile/src/lib.rs`.

### Async work, diagnostics, reflection, and tooling

- Offload CPU work to `AsyncComputeTaskPool` and blocking I/O to `IoTaskPool`. Tasks must not mutate the `World` directly from a worker; return data through a channel or a `CommandQueue`, then apply it on the ECS thread. Poll tasks with `check_ready`; avoid blocking `block_on` in a frame system. References: `async_tasks/async_compute.rs`, `async_channel_pattern.rs`, and `scene/world_serialization.rs`.
- Use `FrameTimeDiagnosticsPlugin`, `EntityCountDiagnosticsPlugin`, system/render diagnostics, `Name`, schedule data, and ambiguity detection to measure a problem before optimizing. Run stress examples in release mode; they are measurement fixtures, not default gameplay architecture. References: `diagnostics/log_diagnostics.rs`, `dev_tools/schedule_data.rs`, `dev_tools/fps_overlay.rs`, and `stress_tests/`.
- Use `Reflect` and `#[reflect(Component)]`/`#[reflect(Resource)]` when types need runtime inspection, dynamic editing, serialization, remote access, or scene/world persistence. Register the types/required type data and keep runtime-only fields skipped or reconstructed with `FromWorld`. References: `reflection/reflection.rs`, `scene/world_serialization.rs`, and `remote/server.rs`.
- BRP/remote support is an opt-in tool surface. Derive the reflection/serialization data required by the remote API and enable the matching feature/plugin only in the environments that need it. Reference: `remote/server.rs` and `remote/integration_test.rs`.

### Features and platform boundaries

- Use `Cargo.toml` features as an intentional profile: the repository's default currently groups 2D, 3D, UI, and audio, while `default_app`, `default_platform`, `2d`, `3d`, `ui`, `scene`, `picking`, and `dev` represent progressively useful subsets. Never copy this list blindly; inspect the target release's feature table.
- For headless/no-window applications disable default features and add only the core features required by the target. For WASM use the target's WebGL2/WebGPU feature combination and account for browser asset paths and audio gesture restrictions. For Android/iOS use the mobile example's crate type, target, and activity/platform setup as the starting point. References: `app/headless.rs`, `no_std/`, `mobile/`, and the platform section of `examples/README.md`.
- `no_std` still needs allocation and only the Bevy subset supported by the target; avoid assuming filesystem, threads, windowing, or GPU facilities are present. Reference: `no_std/README.md` and `no_std/library/`.
- Experimental examples (`shader_advanced`, Solari, meshlets, some widgets/features) are useful implementation evidence but carry compatibility risk. Copy the pattern only after confirming the target version, backend, and feature gate.

## Pitfalls

- A Bevy example from another release can compile-looking but be semantically wrong. Resolve the target version first and prefer the matching checkout/docs.
- A sample's purpose may be an engine-internals demonstration, benchmark, or experimental API rather than a recommended game architecture. Use `stress_tests`, `shader_advanced`, renderer extraction, Solari, meshlets, and experimental widgets as scoped references.
- Systems that happen to run in a useful order in one schedule are not ordered. Make message delivery, transform dependencies, state transitions, and same-frame UI reactions explicit.
- A handle being created does not mean the asset or its recursive dependencies are usable. Keep the handle and observe load state; avoid blocking the main thread.
- A mutable query or `ResMut` can create change detection even when the value is unchanged, and broad mutable access can unnecessarily serialize systems. Narrow the query and use `set_if_neq` where appropriate.
- Background tasks, render-world systems, and exclusive systems have different worlds/threads and ownership rules. Communicate through safe queues/resources rather than reaching across the boundary.
- UI widgets do not supply visual styling, and pointer/keyboard focus is a separate concern from click behavior. Install the relevant picking/focus plugins and style state transitions intentionally.

## Verification

- The target version and feature flags were read from `Cargo.toml`; no API was copied from an unverified release.
- The closest official example was read in full and named in the implementation notes.
- Components/resources/messages/events/observers are registered and owned by an appropriate plugin.
- Schedule ordering, state lifecycle, fixed/virtual time, asset readiness, and async/render-world boundaries are explicit where relevant.
- `cargo fmt --check`, a focused `cargo check`/test, and the smallest relevant runtime/example path pass; visual/platform behavior is checked with the appropriate backend or headless path.
# Dwust

A Dofus-inspired isometric world prototype built with Bevy.

```sh
cargo run
```

- Click the ground to move within the current screen. Player movement finds a route around building footprints and keeps a small clearance; clicking a building routes to its nearest reachable edge instead.
- Click an edge arrow or press a keyboard arrow to walk to that edge, then enter the adjacent screen. On arrival, the screen switches instantly with no camera animation; the player appears at the opposite edge. Clicking the ground cancels pending travel.
- Screens extend in all eight directions, including corner exits; the bottom-left badge shows the current screen coordinates.
- Neutral, translucent edge strips keep all exits clickable, with directional arrows and a subtle hover highlight. Movement defaults to 14 units/second.
- Set the starting speed with `DWUST_MOVE_SPEED`, for example:

```sh
DWUST_MOVE_SPEED=24 cargo run
```

Invalid speeds fall back to 14. All screens share the same neutral gray ground; there are no colored neighbor previews or screen-dependent color changes.

## Configuration

Game settings live in `src/config.rs`. Edit them and restart:

- `GROUND_COLOR`, `SCREEN_EDGE_COLOR`, and `SCREEN_EDGE_HOVER_COLOR` define the neutral world/exit styling.
- Movement speed, player dimensions, world size, view bounds, and window dimensions are in the same file.

Screen transitions have no animation or input lockout.

## Checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Run the real game windowlessly with scripted movement, keyboard transitions,
image-target UI clicks, and 12 checked screenshot checkpoints:

```sh
cargo build
RUN_DIR=$(mktemp -d)
env -u DISPLAY -u WAYLAND_DISPLAY timeout 120s \
  target/debug/dwust --verify-headless "$RUN_DIR/captures" > "$RUN_DIR/run.log" 2>&1
```

The capture directory must not already exist. A graphics adapter (including
software Vulkan/llvmpipe) is still required. Inspect the PNGs and `result.txt`;
`run.log` records assertions and the adapter. This tests game input and UI picking,
not OS input mapping or audio.

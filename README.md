# Dwust

A Dofus-inspired isometric world prototype built with Bevy.

```sh
cargo run
```

- Click the ground to move within the current screen. Player movement finds a route around building footprints and keeps a small clearance; clicking a building routes to its nearest reachable edge instead.
- Click an edge arrow or press a keyboard arrow to walk to that edge, then enter the adjacent screen. On arrival, a fast eased animation moves the camera and player into the adjacent screen; the player ends at the opposite edge. Clicking the ground cancels pending travel.
- Screens extend in all four directions; the top-left HUD shows the current screen coordinates.
- The overlay contains only arrows and screen coordinates. Movement defaults to 14 units/second.
- Set the starting speed with `DWUST_MOVE_SPEED`, for example:

```sh
DWUST_MOVE_SPEED=24 cargo run
```

Invalid speeds fall back to 14. Screens currently share the same checkerboard ground.

## Configuration

Game settings live in `src/config.rs`. Edit them and restart:

- `SCREEN_ANIMATION_ENABLED`: `true` by default; set to `false` for instant screen changes.
- `SCREEN_ANIMATION_DURATION`: duration in seconds (default `0.18`). Zero also makes transitions instant.
- Movement speed, player dimensions, world/tile size, view bounds, and window dimensions are in the same file.

Movement input is ignored during the screen animation to prevent overlapping transitions.

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

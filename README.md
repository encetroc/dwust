# Dwust

A Dofus-inspired isometric world prototype built with Bevy.

```sh
cargo run
```

- Click the ground to move within the current screen. Player movement finds a route around building footprints and keeps a small clearance; clicking a building routes to its nearest reachable edge instead.
- Click an edge arrow or press a keyboard arrow to walk to that edge, then enter the adjacent screen. On arrival, the screen switches instantly with no camera animation; the player appears at the opposite edge. Clicking the ground cancels pending travel.
- Screens extend in all eight directions, including corner exits; the bottom-left badge shows the current screen coordinates.
- Neutral, translucent edge strips keep all exits clickable, with directional arrows and a subtle hover highlight. Movement defaults to 14 units/second.
- Combat spells are loaded from `assets/spells.json`. Select a spell with its number key, then click a valid target in range; `Escape` cancels selection. The configured AP cost is deducted on a successful cast, and AOE affects eligible units around the clicked target cell.
- Set the starting speed with `DWUST_MOVE_SPEED`, for example:

```sh
DWUST_MOVE_SPEED=24 cargo run
```

Invalid speeds fall back to 14. All screens share the same neutral gray ground; there are no colored neighbor previews or screen-dependent color changes.

### Adding spells

Add entries to `assets/spells.json` and restart the game. Each spell has `name`, `description`, `ap_cost`, `range` and `aoe` (Manhattan grid-cell distances), plus `target` (`enemy`, `ally`, or `any`) and an `effects` list. Effects change a stat by the signed `amount`; supported stats are `hp`, `max_hp`, `mp`, `ap`, `damage`, and `armor`. Negative HP damages, positive HP heals up to max health, and non-health stats cannot fall below zero. Spells are selected with keys 1–9 in file order.

Example effect: `{ "stat": "hp", "amount": -10 }` deals 10 damage.

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

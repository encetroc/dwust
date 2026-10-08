# Dwust

A Dofus-inspired isometric world prototype built with Bevy.

```sh
cargo run
```

- Click the ground to move within the current screen. Player movement finds a route around building footprints and keeps a small clearance; clicking a building routes to its nearest reachable edge instead.
- Click an edge arrow or press a keyboard arrow to walk to that edge, then enter the adjacent screen. On arrival, the screen switches instantly with no camera animation; the player appears at the opposite edge. Clicking the ground cancels pending travel.
- Screens extend in all eight directions, including corner exits; the bottom-left badge shows the current screen coordinates.
- Neutral, translucent edge strips keep all exits clickable, with directional arrows and a subtle hover highlight. Movement defaults to 14 units/second.
- Combat spells are loaded from `assets/spells.json`. Select a spell with its number key, then click a valid target in range; `Escape` cancels selection. The configured AP cost is deducted on a successful cast, and AOE affects eligible units around the clicked target cell. Physical and magical damage bonuses are percentages; spell damage is `base_damage * (1 + damage_bonus / 100)` for the matching type.
- Set the starting speed with `DWUST_MOVE_SPEED`, for example:

```sh
DWUST_MOVE_SPEED=24 cargo run
```

Invalid speeds fall back to 14. All screens share the same neutral gray ground; there are no colored neighbor previews or screen-dependent color changes.

### Adding spells

Add entries to `assets/spells.json` and restart the game. Each spell has `name`, `description`, `ap_cost`, `range`, `base_damage`, `damage_type` (`physical` or `magical`), `area` (an odd-sized boolean matrix centered on the target cell), `target` (`enemy`, `ally`, or `any`) and an `effects` list. Damage bonuses use percentage points and are stored as `physical_damage` and `magical_damage`; damage is rounded to whole HP after applying `base_damage * (1 + matching_damage_bonus / 100)`. For example, 10 base magical damage with a 25% magical bonus deals 13 HP. Negative percentages reduce damage, but damage cannot go below zero. Effects change a stat by signed `amount`; supported stats are `hp`, `max_hp`, `mp`, `ap`, `physical_damage`, and `magical_damage`. Positive HP effects heal up to max health. Spells are selected with keys 1–9 in file order.

Example damage spell fields: `"base_damage": 10, "damage_type": "magical"`. Example buff effect: `{ "stat": "magical_damage", "amount": 25 }` adds 25 percentage points.

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

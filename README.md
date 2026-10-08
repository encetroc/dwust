# Dwust

A Dofus-inspired isometric world prototype built with Bevy.

```sh
cargo run
```

- Roam continuously through the world: click the ground to walk to any visible point, or use WASD/arrow keys for direct movement. Mouse movement routes around building footprints; keyboard movement is direct.
- The camera follows the player, and the ground recenters beneath it. There are no screen boundaries, edge exits, or screen transitions. Movement defaults to 14 units/second.
- Combat spells are loaded from `assets/spells.json`. Select a spell with its number key, then click a valid target in range; `Escape` cancels selection. The configured AP cost is deducted on a successful cast, and AOE affects eligible units around the clicked target cell. Physical and magical damage bonuses are percentages; spell damage is `base_damage * (1 + damage_bonus / 100)` for the matching type.
- Set the starting speed with `DWUST_MOVE_SPEED`, for example:

```sh
DWUST_MOVE_SPEED=24 cargo run
```

Invalid speeds fall back to 14. The ground uses a neutral gray material.

### Adding spells

Add entries to `assets/spells.json` and restart the game. Each spell has `name`, `description`, `ap_cost`, `range`, `base_damage`, `damage_type` (`physical` or `magical`), `area` (an odd-sized boolean matrix centered on the target cell), `target` (`enemy`, `ally`, or `any`) and an `effects` list. Damage bonuses use percentage points and are stored as `physical_damage` and `magical_damage`; damage is rounded to whole HP after applying `base_damage * (1 + matching_damage_bonus / 100)`. For example, 10 base magical damage with a 25% magical bonus deals 13 HP. Negative percentages reduce damage, but damage cannot go below zero. Effects change a stat by signed `amount`; supported stats are `hp`, `max_hp`, `mp`, `ap`, `physical_damage`, and `magical_damage`. Positive HP effects heal up to max health. Spells are selected with keys 1–9 in file order.

Example damage spell fields: `"base_damage": 10, "damage_type": "magical"`. Example buff effect: `{ "stat": "magical_damage", "amount": 25 }` adds 25 percentage points.

## Configuration

Game settings live in `src/config.rs`. Edit them and restart:

- `GROUND_COLOR` defines the neutral world styling.
- Movement speed, player dimensions, world size, view bounds, and window dimensions are in the same file.

World movement is continuous; the camera follows the player without screen transitions.

## Checks

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The legacy headless journey script still tests the removed screen-navigation flow and should not be used to verify free-roam behavior. To capture the real combat UI with a windowless render target, run:

```sh
env -u DISPLAY -u WAYLAND_DISPLAY cargo run --features headless-verification -- --verify-combat-ui /tmp/dwust-combat-ui
```

The command saves `combat-ui.png` and `result.txt` in a fresh output directory.

use super::*;
use serde::Deserialize;

const GRID_W: i32 = 8;
const GRID_H: i32 = 6;
const TILE: f32 = 2.0;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum GameMode {
    #[default]
    World,
    Combat,
}

#[derive(Component)]
pub(super) struct WorldObject;

#[derive(Component)]
pub(super) struct WorldEnemy;

#[derive(Component)]
pub(super) struct WorldUi;

#[derive(Component)]
struct CombatObject;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Team {
    Player,
    Enemy,
}

#[derive(Component)]
struct Unit {
    team: Team,
    cell: IVec2,
    hp: i32,
    max_hp: i32,
    mp: i32,
    ap: i32,
    physical_damage: i32,
    magical_damage: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DamageType {
    Physical,
    #[default]
    Magical,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SpellStat {
    Hp,
    MaxHp,
    Mp,
    Ap,
    PhysicalDamage,
    MagicalDamage,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SpellTarget {
    Enemy,
    Ally,
    Any,
}

#[derive(Clone, Debug, Deserialize)]
struct SpellEffect {
    stat: SpellStat,
    amount: i32,
}

#[derive(Clone, Debug, Deserialize)]
struct Spell {
    name: String,
    ap_cost: i32,
    range: i32,
    /// Base damage before the caster's matching percentage bonus is applied.
    #[serde(default)]
    base_damage: i32,
    #[serde(default)]
    damage_type: DamageType,
    /// Row-major footprint centered on the targeted cell; true cells are affected.
    area: Vec<Vec<bool>>,
    target: SpellTarget,
    effects: Vec<SpellEffect>,
}

#[derive(Resource, Default)]
struct SpellBook(Vec<Spell>);

#[derive(Component)]
struct CombatTile {
    cell: IVec2,
    base_color: Color,
}

#[derive(Resource, Default)]
struct TurnOrder {
    entities: Vec<Entity>,
    active: usize,
}

#[derive(Resource)]
struct EnemyTurnDelay(Timer);

#[derive(Resource, Default)]
struct Session {
    spell_selected: Option<usize>,
    camera_transform: Option<Transform>,
    world_enemy: Option<Entity>,
}

#[derive(Component)]
struct CombatStats;

#[derive(Component)]
struct CombatSpells;

#[derive(Component)]
struct CombatTimeline;

pub(super) struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .init_resource::<SpellBook>()
            .add_systems(Startup, load_spells)
            .init_resource::<TurnOrder>()
            .insert_resource(EnemyTurnDelay(Timer::from_seconds(0.65, TimerMode::Once)))
            .add_systems(Startup, spawn_world_enemies)
            .add_systems(OnEnter(GameMode::Combat), enter_combat)
            .add_systems(OnExit(GameMode::Combat), exit_combat)
            .add_systems(
                Update,
                start_combat
                    .after(super::move_player)
                    .run_if(in_state(GameMode::World)),
            )
            .add_systems(
                Update,
                (
                    combat_input,
                    advance_enemy_turn,
                    update_combat_visuals,
                    update_combat_stats,
                    update_combat_spells,
                    update_combat_timeline,
                    finish_if_won,
                )
                    .chain()
                    .run_if(in_state(GameMode::Combat)),
            );
    }
}

fn load_spells(mut book: ResMut<SpellBook>) {
    let contents =
        std::fs::read_to_string("assets/spells.json").expect("failed to read assets/spells.json");
    let spells: Vec<Spell> = serde_json::from_str(&contents)
        .expect("assets/spells.json must contain a valid spell list");
    assert!(
        !spells.is_empty(),
        "assets/spells.json must define at least one spell"
    );
    assert!(
        spells.iter().all(|spell| {
            spell.ap_cost >= 0
                && spell.range >= 0
                && spell.base_damage >= 0
                && valid_area(&spell.area)
        }),
        "spell AP cost and range must be non-negative; area must be a non-empty odd-sized matrix with a true center"
    );
    book.0 = spells;
}

fn spawn_world_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viewport: Res<GameViewport>,
) {
    let mesh = meshes.add(Sphere::new(0.85).mesh().uv(24, 16));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.06, 0.04),
        emissive: LinearRgba::new(0.12, 0.0, 0.0, 1.0),
        ..default()
    });
    let half = screen_half_extents(viewport.size);
    for screen in [IVec2::ZERO, IVec2::X, IVec2::NEG_X, IVec2::Y, IVec2::NEG_Y] {
        let center = SCREEN_RIGHT * (screen.x as f32 * half.x * 2.0)
            - SCREEN_UP * (screen.y as f32 * half.y * 2.0);
        let position = center + SCREEN_UP * (half.y * 0.65) + Vec3::Y * 0.85;
        commands.spawn((
            WorldObject,
            WorldEnemy,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position),
            Name::new("World Enemy"),
        ));
    }
}

fn start_combat(
    mut session: ResMut<Session>,
    mut next: ResMut<NextState<GameMode>>,
    enemies: Query<&Transform, With<WorldEnemy>>,
    mut player: Single<(&Transform, &mut Player)>,
) {
    let player_transform = player.0;
    let player_state = &mut *player.1;
    let Some(enemy) = player_state.approach_enemy else {
        return;
    };
    let Ok(enemy_transform) = enemies.get(enemy) else {
        player_state.approach_enemy = None;
        return;
    };
    let player_position = Vec2::new(
        player_transform.translation.x,
        player_transform.translation.z,
    );
    let enemy_position = Vec2::new(enemy_transform.translation.x, enemy_transform.translation.z);
    let contact_distance = super::PLAYER_RADIUS + 0.85;
    if player_position.distance_squared(enemy_position) <= contact_distance * contact_distance {
        player_state.destination = None;
        player_state.path.clear();
        player_state.travel = None;
        player_state.approach_enemy = None;
        session.world_enemy = Some(enemy);
        next.set(GameMode::Combat);
    }
}

pub(super) fn ray_hits_enemy(
    ray: Ray3d,
    enemies: &Query<(Entity, &GlobalTransform), With<WorldEnemy>>,
) -> Option<(Entity, Vec3)> {
    enemies.iter().find_map(|(entity, transform)| {
        ray_sphere_hit(ray, transform.translation(), 0.95)
            .then_some((entity, transform.translation()))
    })
}

fn ray_sphere_hit(ray: Ray3d, center: Vec3, radius: f32) -> bool {
    let offset = ray.origin - center;
    let b = offset.dot(*ray.direction);
    let c = offset.length_squared() - radius * radius;
    b * b - c >= 0.0 && -b + (b * b - c).sqrt() >= 0.0
}

fn enter_combat(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut turn_order: ResMut<TurnOrder>,
    mut enemy_turn_delay: ResMut<EnemyTurnDelay>,
    mut world: Query<&mut Visibility, Or<(With<WorldObject>, With<WorldUi>)>>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    session.spell_selected = None;
    session.camera_transform = Some(**camera);
    for mut visibility in &mut world {
        *visibility = Visibility::Hidden;
    }

    let camera_focus = Vec3::ZERO;
    **camera = Transform::from_xyz(16.0, 22.0, 18.0).looking_at(camera_focus, Vec3::Y);

    turn_order.entities.clear();
    turn_order.active = 0;
    enemy_turn_delay.0.reset();
    for x in 0..GRID_W {
        for z in 0..GRID_H {
            let cell = IVec2::new(x, z);
            let position = cell_to_world(cell);
            let base_color = if (x + z) & 1 == 0 {
                Color::srgb(0.30, 0.34, 0.25)
            } else {
                Color::srgb(0.39, 0.42, 0.31)
            };
            commands.spawn((
                CombatObject,
                DespawnOnExit(GameMode::Combat),
                CombatTile { cell, base_color },
                Mesh3d(meshes.add(Cuboid::new(TILE * 0.98, 0.12, TILE * 0.98))),
                MeshMaterial3d(materials.add(base_color)),
                Transform::from_translation(position - Vec3::Y * 0.06),
            ));
        }
    }
    turn_order.entities.push(spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Player,
        IVec2::new(1, 2),
        100,
        Color::srgb(0.15, 0.38, 0.95),
    ));
    turn_order.entities.push(spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Enemy,
        IVec2::new(6, 1),
        20,
        Color::srgb(0.9, 0.06, 0.04),
    ));
    turn_order.entities.push(spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Enemy,
        IVec2::new(6, 4),
        20,
        Color::srgb(0.9, 0.06, 0.04),
    ));
    super::hud::spawn_hud_window(
        &mut commands,
        "HP / MP / AP",
        CombatStats,
        Vec2::new(16.0, 192.0),
        Vec2::new(256.0, 80.0),
        "",
        16.0,
    );
    super::hud::spawn_hud_window(
        &mut commands,
        "Spells",
        CombatSpells,
        Vec2::new(272.0, 192.0),
        Vec2::new(512.0, 208.0),
        "",
        16.0,
    );
    super::hud::spawn_hud_window(
        &mut commands,
        "Timeline",
        CombatTimeline,
        Vec2::new(16.0, 416.0),
        Vec2::new(768.0, 64.0),
        "",
        16.0,
    );
}

fn spawn_unit(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    team: Team,
    cell: IVec2,
    hp: i32,
    color: Color,
) -> Entity {
    commands
        .spawn((
            CombatObject,
            DespawnOnExit(GameMode::Combat),
            Unit {
                team,
                cell,
                hp,
                max_hp: hp,
                mp: if team == Team::Player { 3 } else { 0 },
                ap: if team == Team::Player { 6 } else { 0 },
                physical_damage: 0,
                magical_damage: 0,
            },
            Mesh3d(meshes.add(Sphere::new(0.72).mesh().uv(24, 16))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_translation(cell_to_world(cell) + Vec3::Y * 0.85),
        ))
        .id()
}

fn exit_combat(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut world: Query<&mut Visibility, Or<(With<WorldObject>, With<WorldUi>)>>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    for mut visibility in &mut world {
        *visibility = Visibility::Visible;
    }
    if let Some(transform) = session.camera_transform.take() {
        **camera = transform;
    }
    if let Some(enemy) = session.world_enemy.take() {
        commands.entity(enemy).despawn();
    }
}

fn cell_to_world(cell: IVec2) -> Vec3 {
    Vec3::new(
        (cell.x as f32 - (GRID_W - 1) as f32 / 2.0) * TILE,
        0.0,
        (cell.y as f32 - (GRID_H - 1) as f32 / 2.0) * TILE,
    )
}

fn world_to_cell(point: Vec3) -> IVec2 {
    IVec2::new(
        (point.x / TILE + (GRID_W - 1) as f32 / 2.0).round() as i32,
        (point.z / TILE + (GRID_H - 1) as f32 / 2.0).round() as i32,
    )
}

fn combat_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    viewport: Res<GameViewport>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    book: Res<SpellBook>,
    mut session: ResMut<Session>,
    mut turn_order: ResMut<TurnOrder>,
    mut next: ResMut<NextState<GameMode>>,
    mut units: Query<(Entity, &mut Unit, &mut Transform)>,
) {
    let is_player_turn = turn_order
        .entities
        .get(turn_order.active)
        .is_some_and(|active| turn_order.entities.first() == Some(active));
    if keyboard.just_pressed(KeyCode::Escape) {
        session.spell_selected = None;
    }
    if keyboard.just_pressed(KeyCode::Space) && is_player_turn && !turn_order.entities.is_empty() {
        turn_order.active = (turn_order.active + 1) % turn_order.entities.len();
        session.spell_selected = None;
        return;
    }
    if !is_player_turn {
        return;
    }
    let spell_keys = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    if let Some(index) = spell_keys
        .iter()
        .position(|key| keyboard.just_pressed(*key))
    {
        session.spell_selected = (index < book.0.len()).then_some(index);
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = viewport.cursor else {
        return;
    };
    let (camera, transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    let Some(point) = ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
    else {
        return;
    };
    let cell = world_to_cell(point);
    if cell.x < 0 || cell.x >= GRID_W || cell.y < 0 || cell.y >= GRID_H {
        return;
    }

    let Some((player_entity, player_cell, player_ap, player_mp)) =
        units.iter().find_map(|(entity, unit, _)| {
            (unit.team == Team::Player).then_some((entity, unit.cell, unit.ap, unit.mp))
        })
    else {
        return;
    };
    let distance = (cell.x - player_cell.x).abs() + (cell.y - player_cell.y).abs();
    if let Some(index) = session.spell_selected {
        let Some(spell) = book.0.get(index) else {
            session.spell_selected = None;
            return;
        };
        if distance > spell.range || player_ap < spell.ap_cost {
            return;
        }
        let spell = spell.clone();
        let caster_bonus = units
            .get(player_entity)
            .map_or(0, |(_, unit, _)| damage_bonus(unit, spell.damage_type));
        let dealt_damage = calculate_damage(spell.base_damage, caster_bonus);
        if let Ok((_, mut player, _)) = units.get_mut(player_entity) {
            player.ap -= spell.ap_cost;
        }
        for (_, mut unit, mut transform) in &mut units {
            let offset = unit.cell - cell;
            if unit.hp <= 0
                || !area_contains(&spell.area, offset)
                || !target_matches(spell.target, unit.team)
            {
                continue;
            }
            if dealt_damage > 0 {
                unit.hp = unit.hp.saturating_sub(dealt_damage).max(0);
            }
            for effect in &spell.effects {
                apply_effect(&mut unit, effect);
            }
            if unit.hp <= 0 {
                transform.scale = Vec3::splat(0.001);
            }
        }
        session.spell_selected = None;
        if units
            .iter()
            .all(|(_, unit, _)| unit.team != Team::Enemy || unit.hp <= 0)
        {
            next.set(GameMode::World);
        }
        return;
    }
    if distance == 0 || distance > player_mp {
        return;
    }
    if units
        .iter()
        .any(|(_, unit, _)| unit.hp > 0 && unit.cell == cell)
    {
        return;
    }
    if let Ok((_, mut player, mut transform)) = units.get_mut(player_entity) {
        player.mp -= distance;
        player.cell = cell;
        transform.translation = cell_to_world(cell) + Vec3::Y * 0.85;
    }
}

fn valid_area(area: &[Vec<bool>]) -> bool {
    let Some(width) = area.first().map(Vec::len) else {
        return false;
    };
    width > 0
        && area.len() % 2 == 1
        && width % 2 == 1
        && area.iter().all(|row| row.len() == width)
        && area[area.len() / 2][width / 2]
}

fn area_contains(area: &[Vec<bool>], offset: IVec2) -> bool {
    let Some(width) = area.first().map(Vec::len) else {
        return false;
    };
    let col = offset.x + (width / 2) as i32;
    let row = offset.y + (area.len() / 2) as i32;
    row >= 0
        && row < area.len() as i32
        && col >= 0
        && col < width as i32
        && area[row as usize][col as usize]
}

fn area_cells(area: &[Vec<bool>], center: IVec2) -> Vec<IVec2> {
    let row_center = (area.len() / 2) as i32;
    let col_center = area.first().map_or(0, |row| (row.len() / 2) as i32);
    area.iter()
        .enumerate()
        .flat_map(|(row, cells)| {
            cells.iter().enumerate().filter_map(move |(col, affected)| {
                affected.then_some(
                    center + IVec2::new(col as i32 - col_center, row as i32 - row_center),
                )
            })
        })
        .collect()
}

fn target_matches(target: SpellTarget, team: Team) -> bool {
    match target {
        SpellTarget::Enemy => team == Team::Enemy,
        SpellTarget::Ally => team == Team::Player,
        SpellTarget::Any => true,
    }
}

fn damage_bonus(unit: &Unit, damage_type: DamageType) -> i32 {
    match damage_type {
        DamageType::Physical => unit.physical_damage,
        DamageType::Magical => unit.magical_damage,
    }
}

fn calculate_damage(base_damage: i32, damage_bonus_percent: i32) -> i32 {
    (base_damage.max(0) as f64 * (1.0 + damage_bonus_percent as f64 / 100.0))
        .round()
        .clamp(0.0, i32::MAX as f64) as i32
}

fn apply_effect(unit: &mut Unit, effect: &SpellEffect) {
    match effect.stat {
        SpellStat::Hp => unit.hp = unit.hp.saturating_add(effect.amount).clamp(0, unit.max_hp),
        SpellStat::MaxHp => {
            unit.max_hp = unit.max_hp.saturating_add(effect.amount).max(1);
            unit.hp = unit.hp.min(unit.max_hp);
        }
        SpellStat::Mp => unit.mp = unit.mp.saturating_add(effect.amount).max(0),
        SpellStat::Ap => unit.ap = unit.ap.saturating_add(effect.amount).max(0),
        SpellStat::PhysicalDamage => {
            unit.physical_damage = unit.physical_damage.saturating_add(effect.amount)
        }
        SpellStat::MagicalDamage => {
            unit.magical_damage = unit.magical_damage.saturating_add(effect.amount)
        }
    }
}

fn advance_enemy_turn(
    time: Res<Time>,
    mut delay: ResMut<EnemyTurnDelay>,
    mut turn_order: ResMut<TurnOrder>,
    mut session: ResMut<Session>,
    mut units: Query<&mut Unit>,
) {
    let Some(&active) = turn_order.entities.get(turn_order.active) else {
        return;
    };
    let is_enemy_turn = units.get(active).is_ok_and(|unit| unit.team == Team::Enemy);
    if !is_enemy_turn {
        delay.0.reset();
        return;
    }
    let active_is_alive = units.get(active).is_ok_and(|unit| unit.hp > 0);
    if active_is_alive && !delay.0.tick(time.delta()).is_finished() {
        return;
    }
    turn_order.active = (turn_order.active + 1) % turn_order.entities.len();
    delay.0.reset();
    if turn_order.active == 0
        && let Some(&player) = turn_order.entities.first()
        && let Ok(mut unit) = units.get_mut(player)
    {
        unit.mp = 3;
        unit.ap = 6;
    }
    session.spell_selected = None;
}

fn update_combat_visuals(
    session: Res<Session>,
    book: Res<SpellBook>,
    turn_order: Res<TurnOrder>,
    viewport: Res<GameViewport>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    units: Query<&Unit>,
    mut tiles: Query<(&CombatTile, &MeshMaterial3d<StandardMaterial>)>,
    mut appearances: Query<(&Unit, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_data = units
        .iter()
        .find(|unit| unit.team == Team::Player)
        .map(|unit| (unit.cell, unit.mp, unit.ap));
    let player_cell = player_data.map(|(cell, _, _)| cell);
    let mut hovered_cell = None;
    if let Some(cursor) = viewport.cursor {
        let (camera, transform) = *camera;
        if let Ok(ray) = camera.viewport_to_world(transform, cursor)
            && let Some(point) =
                ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
        {
            let cell = world_to_cell(point);
            if cell.x >= 0 && cell.x < GRID_W && cell.y >= 0 && cell.y < GRID_H {
                hovered_cell = Some(cell);
            }
        }
    }
    let hovered_unit =
        hovered_cell.filter(|cell| units.iter().any(|unit| unit.hp > 0 && unit.cell == *cell));
    let player_turn = turn_order
        .entities
        .get(turn_order.active)
        .is_some_and(|active| {
            // The player is the first entity in the turn order.
            turn_order.entities.first() == Some(active)
        });
    let selected_spell = session.spell_selected.and_then(|index| book.0.get(index));
    let preview_area = selected_spell.and_then(|spell| {
        let (origin, center) = (player_cell?, hovered_cell?);
        let distance = (center.x - origin.x).abs() + (center.y - origin.y).abs();
        let enough_ap = player_data.is_some_and(|(_, _, ap)| ap >= spell.ap_cost);
        (player_turn && distance <= spell.range && enough_ap)
            .then(|| area_cells(&spell.area, center))
    });
    let mut path = Vec::new();
    if player_turn
        && session.spell_selected.is_none()
        && let (Some(start), Some(end)) = (player_cell, hovered_cell)
    {
        let distance = (end.x - start.x).abs() + (end.y - start.y).abs();
        let occupied = units
            .iter()
            .any(|unit| unit.team == Team::Enemy && unit.hp > 0 && unit.cell == end);
        let player_mp = player_data.map_or(0, |(_, mp, _)| mp);
        if distance > 0 && distance <= player_mp && !occupied {
            let mut cell = start;
            while cell.x != end.x {
                cell.x += (end.x - cell.x).signum();
                path.push(cell);
            }
            while cell.y != end.y {
                cell.y += (end.y - cell.y).signum();
                path.push(cell);
            }
        }
    }
    for (unit, material_handle) in &mut appearances {
        let affected = unit.hp > 0
            && preview_area
                .as_ref()
                .is_some_and(|cells| cells.contains(&unit.cell))
            && selected_spell.is_some_and(|spell| target_matches(spell.target, unit.team));
        let color = if affected {
            Color::srgb(1.0, 0.45, 0.12)
        } else if unit.hp > 0 && hovered_unit == Some(unit.cell) {
            Color::srgb(1.0, 0.78, 0.12)
        } else if unit.team == Team::Player {
            Color::srgb(0.15, 0.38, 0.95)
        } else {
            Color::srgb(0.9, 0.06, 0.04)
        };
        if let Some(mut material) = materials.get_mut(&material_handle.0)
            && material.base_color != color
        {
            material.base_color = color;
        }
    }
    for (tile, material_handle) in &mut tiles {
        let color = if preview_area
            .as_ref()
            .is_some_and(|cells| cells.contains(&tile.cell))
        {
            Color::srgb(0.95, 0.35, 0.18)
        } else if hovered_unit == Some(tile.cell) {
            Color::srgb(1.0, 0.78, 0.12)
        } else if player_turn
            && session.spell_selected.is_some_and(|index| {
                book.0.get(index).is_some_and(|spell| {
                    player_data.is_some_and(|(origin, _, ap)| {
                        ap >= spell.ap_cost
                            && (tile.cell.x - origin.x).abs() + (tile.cell.y - origin.y).abs()
                                <= spell.range
                    })
                })
            })
        {
            Color::srgb(0.20, 0.55, 0.85)
        } else if path.contains(&tile.cell) {
            Color::srgb(0.25, 0.72, 0.35)
        } else {
            tile.base_color
        };
        if let Some(mut material) = materials.get_mut(&material_handle.0)
            && material.base_color != color
        {
            material.base_color = color;
        }
    }
}

fn update_combat_stats(mut stats: Single<&mut Text, With<CombatStats>>, units: Query<&Unit>) {
    let player = units.iter().find(|unit| unit.team == Team::Player);
    let hp = player.map_or(0, |unit| unit.hp.max(0));
    let mp = player.map_or(0, |unit| unit.mp);
    let ap = player.map_or(0, |unit| unit.ap);
    let physical = player.map_or(0, |unit| unit.physical_damage);
    let magical = player.map_or(0, |unit| unit.magical_damage);
    stats.0 =
        format!("HP {hp}    MP {mp}    AP {ap}\nPhysical +{physical}%    Magical +{magical}%");
}

fn update_combat_spells(
    session: Res<Session>,
    book: Res<SpellBook>,
    mut text: Single<&mut Text, With<CombatSpells>>,
) {
    text.0 = book
        .0
        .iter()
        .enumerate()
        .map(|(index, spell)| {
            let selected = if session.spell_selected == Some(index) {
                " [SELECTED]"
            } else {
                ""
            };
            let damage = if spell.base_damage > 0 {
                format!(
                    " — {} AP, {} {} ({})",
                    spell.ap_cost,
                    spell.base_damage,
                    match spell.damage_type {
                        DamageType::Physical => "physical",
                        DamageType::Magical => "magical",
                    },
                    spell.range
                )
            } else {
                format!(" — {} AP, range {}", spell.ap_cost, spell.range)
            };
            format!("[{}] {}{}{}", index + 1, spell.name, damage, selected)
        })
        .collect::<Vec<_>>()
        .join("\n");
}

fn update_combat_timeline(
    turn_order: Res<TurnOrder>,
    units: Query<&Unit>,
    mut timeline: Single<&mut Text, With<CombatTimeline>>,
) {
    let entries = turn_order
        .entities
        .iter()
        .enumerate()
        .map(|(index, entity)| {
            let label = if index == 0 {
                "Player"
            } else {
                match units.get(*entity).map(|unit| unit.team) {
                    Ok(Team::Enemy) => {
                        if index == 1 {
                            "Enemy 1"
                        } else {
                            "Enemy 2"
                        }
                    }
                    _ => "Entity",
                }
            };
            if index == turn_order.active {
                format!("[* {label}]")
            } else {
                format!("  {label}  ")
            }
        })
        .collect::<Vec<_>>();
    timeline.0 = format!("TURN ORDER   {}", entries.join(" > "));
}

fn finish_if_won(units: Query<&Unit, With<CombatObject>>, mut next: ResMut<NextState<GameMode>>) {
    if units
        .iter()
        .all(|unit| unit.team != Team::Enemy || unit.hp <= 0)
    {
        next.set(GameMode::World);
    }
}

#[cfg(test)]
mod spell_tests {
    use super::*;

    fn test_unit() -> Unit {
        Unit {
            team: Team::Player,
            cell: IVec2::ZERO,
            hp: 40,
            max_hp: 100,
            mp: 3,
            ap: 6,
            physical_damage: 0,
            magical_damage: 0,
        }
    }

    #[test]
    fn configured_spells_deserialize_and_have_valid_costs_and_distances() {
        let spells: Vec<Spell> = serde_json::from_str(include_str!("../assets/spells.json"))
            .expect("the example spell configuration should deserialize");
        assert!(!spells.is_empty());
        assert!(spells.iter().all(|spell| {
            spell.ap_cost >= 0
                && spell.range >= 0
                && spell.base_damage >= 0
                && valid_area(&spell.area)
        }));
    }

    #[test]
    fn area_matrix_maps_affected_cells_around_its_center() {
        let area = vec![
            vec![false, true, false],
            vec![true, true, true],
            vec![false, true, false],
        ];
        let center = IVec2::new(3, 2);
        assert!(valid_area(&area));
        assert!(area_contains(&area, IVec2::ZERO));
        assert!(area_contains(&area, IVec2::X));
        assert!(!area_contains(&area, IVec2::ONE));
        assert_eq!(area_cells(&area, center).len(), 5);
    }

    #[test]
    fn damage_uses_the_matching_percentage_bonus_and_rounds_to_whole_hp() {
        let mut unit = test_unit();
        unit.physical_damage = 25;
        unit.magical_damage = 50;
        assert_eq!(
            calculate_damage(20, damage_bonus(&unit, DamageType::Physical)),
            25
        );
        assert_eq!(
            calculate_damage(20, damage_bonus(&unit, DamageType::Magical)),
            30
        );
        assert_eq!(calculate_damage(9, 10), 10);
        assert_eq!(calculate_damage(20, -150), 0);
    }

    #[test]
    fn spell_effects_damage_heal_and_adjust_other_stats_safely() {
        let mut unit = test_unit();
        apply_effect(
            &mut unit,
            &SpellEffect {
                stat: SpellStat::Hp,
                amount: -50,
            },
        );
        assert_eq!(unit.hp, 0);
        apply_effect(
            &mut unit,
            &SpellEffect {
                stat: SpellStat::Hp,
                amount: 200,
            },
        );
        assert_eq!(unit.hp, unit.max_hp);
        apply_effect(
            &mut unit,
            &SpellEffect {
                stat: SpellStat::Ap,
                amount: -20,
            },
        );
        apply_effect(
            &mut unit,
            &SpellEffect {
                stat: SpellStat::PhysicalDamage,
                amount: 40,
            },
        );
        assert_eq!(unit.ap, 0);
        assert_eq!(unit.physical_damage, 40);
    }
}

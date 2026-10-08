use super::*;

const GRID_W: i32 = 8;
const GRID_H: i32 = 6;
const TILE: f32 = 2.0;
const SPELL_RANGE: i32 = 3;
const SPELL_AP: u8 = 3;
const SPELL_DAMAGE: i32 = 10;

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
}

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
    mp: u8,
    ap: u8,
    spell_selected: bool,
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
    session.mp = 3;
    session.ap = 6;
    session.spell_selected = false;
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
        Vec2::new(256.0, 64.0),
        "",
        16.0,
    );
    super::hud::spawn_hud_window(
        &mut commands,
        "Spells",
        CombatSpells,
        Vec2::new(272.0, 192.0),
        Vec2::new(512.0, 96.0),
        "",
        16.0,
    );
    super::hud::spawn_hud_window(
        &mut commands,
        "Timeline",
        CombatTimeline,
        Vec2::new(16.0, 288.0),
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
            Unit { team, cell, hp },
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
        session.spell_selected = false;
    }
    if keyboard.just_pressed(KeyCode::Space) && is_player_turn && !turn_order.entities.is_empty() {
        turn_order.active = (turn_order.active + 1) % turn_order.entities.len();
        session.spell_selected = false;
        return;
    }
    if !is_player_turn {
        return;
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        session.spell_selected = true;
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

    let player_cell = units
        .iter()
        .find_map(|(_, unit, _)| (unit.team == Team::Player).then_some(unit.cell));
    let Some(player_cell) = player_cell else {
        return;
    };
    let distance = (cell.x - player_cell.x).abs() + (cell.y - player_cell.y).abs();
    let target = units
        .iter()
        .position(|(_, unit, _)| unit.team == Team::Enemy && unit.hp > 0 && unit.cell == cell);
    if session.spell_selected {
        if let Some(target) = target
            && distance <= SPELL_RANGE
            && session.ap >= SPELL_AP
        {
            session.ap -= SPELL_AP;
            session.spell_selected = false;
            if let Some((_, mut unit, mut transform)) = units.iter_mut().nth(target) {
                unit.hp -= SPELL_DAMAGE;
                if unit.hp <= 0 {
                    transform.scale = Vec3::splat(0.001);
                }
            }
            if units
                .iter()
                .all(|(_, unit, _)| unit.team != Team::Enemy || unit.hp <= 0)
            {
                next.set(GameMode::World);
            }
        }
        return;
    }
    if target.is_some() || distance == 0 || distance > session.mp as i32 {
        return;
    }
    if units
        .iter()
        .any(|(_, unit, _)| unit.team == Team::Enemy && unit.cell == cell && unit.hp > 0)
    {
        return;
    }
    session.mp -= distance as u8;
    if let Some((_, mut unit, mut transform)) = units
        .iter_mut()
        .find(|(_, unit, _)| unit.team == Team::Player)
    {
        unit.cell = cell;
        transform.translation = cell_to_world(cell) + Vec3::Y * 0.85;
    }
}

fn advance_enemy_turn(
    time: Res<Time>,
    mut delay: ResMut<EnemyTurnDelay>,
    mut turn_order: ResMut<TurnOrder>,
    mut session: ResMut<Session>,
    units: Query<&Unit>,
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
    if turn_order.active == 0 {
        session.mp = 3;
        session.ap = 6;
    }
    session.spell_selected = false;
}

fn update_combat_visuals(
    session: Res<Session>,
    turn_order: Res<TurnOrder>,
    viewport: Res<GameViewport>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    units: Query<&Unit>,
    mut tiles: Query<(&CombatTile, &MeshMaterial3d<StandardMaterial>)>,
    mut appearances: Query<(&Unit, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_cell = units
        .iter()
        .find_map(|unit| (unit.team == Team::Player).then_some(unit.cell));
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
    let mut path = Vec::new();
    if player_turn
        && !session.spell_selected
        && let (Some(start), Some(end)) = (player_cell, hovered_cell)
    {
        let distance = (end.x - start.x).abs() + (end.y - start.y).abs();
        let occupied = units
            .iter()
            .any(|unit| unit.team == Team::Enemy && unit.hp > 0 && unit.cell == end);
        if distance > 0 && distance <= session.mp as i32 && !occupied {
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
        let color = if unit.hp > 0 && hovered_unit == Some(unit.cell) {
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
        let color = if hovered_unit == Some(tile.cell) {
            Color::srgb(1.0, 0.78, 0.12)
        } else if player_turn
            && session.spell_selected
            && player_cell.is_some_and(|origin| {
                (tile.cell.x - origin.x).abs() + (tile.cell.y - origin.y).abs() <= SPELL_RANGE
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

fn update_combat_stats(
    session: Res<Session>,
    mut stats: Single<&mut Text, With<CombatStats>>,
    units: Query<&Unit>,
) {
    let hp = units
        .iter()
        .find(|unit| unit.team == Team::Player)
        .map_or(0, |player| player.hp.max(0));
    stats.0 = format!("HP {hp}    MP {}    AP {}", session.mp, session.ap);
}

fn update_combat_spells(session: Res<Session>, mut spells: Single<&mut Text, With<CombatSpells>>) {
    let selection = if session.spell_selected {
        " [SELECTED]"
    } else {
        ""
    };
    spells.0 = format!(
        "SPELLS\n[1] Fireball - {SPELL_DAMAGE} damage, {SPELL_AP} AP, range {SPELL_RANGE}{selection}"
    );
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

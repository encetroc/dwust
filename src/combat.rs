use super::*;
use bevy::{picking::hover::Hovered, ui::Pressed, ui_widgets::Button as UiButton};
use serde::{Deserialize, Serialize};

const GRID_W: i32 = 10;
const GRID_H: i32 = 8;
const COMBAT_MAP_PATH: &str = "assets/combat_maps/stone_causeway.json";
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
    equipment_modifiers: super::inventory::EquipmentModifiers,
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
    #[serde(default)]
    description: String,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Terrain {
    Floor,
    Obstacle,
    Hole,
}

#[derive(Component)]
struct CombatTile {
    cell: IVec2,
    base_color: Color,
    terrain: Terrain,
}

#[derive(Component)]
struct MovementRangeTile(IVec2);

#[derive(Component)]
struct CombatPreparation;

#[derive(Component)]
struct CombatPhasePanel;

#[derive(Resource, Default)]
struct PreparationPhase(bool);

#[derive(Resource, Debug, Deserialize, Serialize)]
struct CombatMap {
    id: String,
    name: String,
    width: i32,
    height: i32,
    terrain: Vec<String>,
    player_spawns: Vec<[i32; 2]>,
    enemy_spawns: Vec<[i32; 2]>,
    holes: Vec<[i32; 2]>,
}

impl CombatMap {
    fn load() -> Self {
        let contents = std::fs::read_to_string(COMBAT_MAP_PATH)
            .unwrap_or_else(|error| panic!("failed to read {COMBAT_MAP_PATH}: {error}"));
        let map: Self = serde_json::from_str(&contents).unwrap_or_else(|error| {
            panic!("{COMBAT_MAP_PATH} must contain a valid combat map: {error}")
        });
        assert!(
            map.width == GRID_W && map.height == GRID_H,
            "combat map dimensions must be {GRID_W}x{GRID_H}"
        );
        assert!(
            map.terrain.len() == map.height as usize
                && map
                    .terrain
                    .iter()
                    .all(|row| row.chars().count() == map.width as usize),
            "combat map terrain rows must match its dimensions"
        );
        assert!(
            !map.player_spawns.is_empty() && map.enemy_spawns.len() >= 2,
            "combat map needs player and enemy spawn cells"
        );
        for cell in map
            .player_spawns
            .iter()
            .chain(&map.enemy_spawns)
            .chain(&map.holes)
        {
            assert!(
                cell[0] >= 0 && cell[0] < map.width && cell[1] >= 0 && cell[1] < map.height,
                "combat map contains out-of-bounds cell {cell:?}"
            );
        }
        map
    }

    fn terrain_at(&self, cell: IVec2) -> Terrain {
        if self.holes.contains(&[cell.x, cell.y]) {
            return Terrain::Hole;
        }
        match self.terrain[cell.y as usize]
            .chars()
            .nth(cell.x as usize)
            .unwrap()
        {
            '#' => Terrain::Obstacle,
            _ => Terrain::Floor,
        }
    }

    fn player_cells(&self) -> Vec<IVec2> {
        self.player_spawns
            .iter()
            .map(|p| IVec2::new(p[0], p[1]))
            .collect()
    }

    fn enemy_cells(&self) -> Vec<IVec2> {
        self.enemy_spawns
            .iter()
            .map(|p| IVec2::new(p[0], p[1]))
            .collect()
    }

    fn is_player_spawn(&self, cell: IVec2) -> bool {
        self.player_spawns.contains(&[cell.x, cell.y])
    }

    fn is_enemy_spawn(&self, cell: IVec2) -> bool {
        self.enemy_spawns.contains(&[cell.x, cell.y])
    }

    fn walkable(&self, cell: IVec2) -> bool {
        cell.x >= 0
            && cell.x < self.width
            && cell.y >= 0
            && cell.y < self.height
            && self.terrain_at(cell) == Terrain::Floor
    }
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

#[derive(Component, Clone, Copy)]
struct CombatStatCell(CombatStat);

#[derive(Clone, Copy)]
enum CombatStat {
    Hp,
    Mp,
    Ap,
}

#[derive(Component)]
struct CombatSpells;

#[derive(Component)]
struct AbilityIcon(usize);

#[derive(Component)]
struct AbilityInfoContextMenu;

#[derive(Component)]
struct AbilityInfoAction(usize);

#[derive(Component)]
struct CombatTimeline;

#[derive(Component)]
struct TimelineEntryName(Entity);

#[derive(Component)]
struct TimelineEntryHealth(Entity);

#[derive(Component)]
struct TimelineHealthBar(Entity);

pub(super) struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .init_resource::<PreparationPhase>()
            .insert_resource(CombatMap::load())
            .init_resource::<SpellBook>()
            .add_systems(Startup, (load_spells, spawn_shared_combat_modules).chain())
            .init_resource::<TurnOrder>()
            .insert_resource(EnemyTurnDelay(Timer::from_seconds(0.65, TimerMode::Once)))
            .add_systems(Startup, spawn_world_enemies)
            .add_systems(Update, (open_ability_context_menu, select_ability_info))
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
                    update_equipment_stats,
                    combat_input,
                    advance_enemy_turn,
                    update_combat_visuals,
                    update_combat_stats,
                    update_combat_spells,
                    update_combat_timeline,
                    finish_if_won,
                    update_combat_preparation,
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

fn spawn_shared_combat_modules(mut commands: Commands, book: Res<SpellBook>) {
    let abilities_grid = super::hud_layout::HudGrid::fixed_cells(
        10,
        2,
        36.0,
        8.0,
        UiRect::new(px(8), px(28), px(8), px(8)),
    );
    let (_, abilities_content) = super::hud::spawn_hud_window(
        &mut commands,
        "Abilities",
        Some(KeyCode::KeyP),
        CombatSpells,
        Vec2::new(352.0, 192.0),
        abilities_grid,
        super::hud::HudModuleType::Both,
        true,
    );
    commands.entity(abilities_content).with_children(|grid| {
        let mut used_letters = Vec::new();
        for (index, spell) in book.0.iter().enumerate() {
            let letter = spell
                .name
                .chars()
                .find(|character| {
                    character.is_ascii_alphabetic()
                        && !used_letters.contains(&character.to_ascii_uppercase())
                })
                .unwrap_or('?')
                .to_ascii_uppercase();
            used_letters.push(letter);
            let color = ability_color(spell);
            grid.spawn((
                AbilityIcon(index),
                UiButton,
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(color),
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.35)),
            ))
            .with_children(|icon| {
                icon.spawn((
                    Text::new(letter.to_string()),
                    TextFont {
                        font_size: FontSize::Px(18.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        }
    });
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
    map: Res<CombatMap>,
    mut preparation: ResMut<PreparationPhase>,
    equipment: Res<super::inventory::CurrentEquipmentModifiers>,
) {
    preparation.0 = true;
    session.spell_selected = None;
    session.camera_transform = Some(**camera);
    for mut visibility in &mut world {
        *visibility = Visibility::Hidden;
    }

    let camera_focus = Vec3::ZERO;
    **camera = Transform::from_xyz(16.0, 22.0, 18.0).looking_at(camera_focus, Vec3::Y);

    let movement_range_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.72, 0.32, 0.45),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        // Keep the overlay at the grid surface while making it pass the depth
        // test in front of units and terrain props.
        depth_bias: 10_000.0,
        ..default()
    });
    turn_order.entities.clear();
    turn_order.active = 0;
    enemy_turn_delay.0.reset();
    for x in 0..GRID_W {
        for z in 0..GRID_H {
            let cell = IVec2::new(x, z);
            let position = cell_to_world(cell);
            let terrain = map.terrain_at(cell);
            let base_color = match terrain {
                Terrain::Floor if (x + z) & 1 == 0 => Color::srgb(0.22, 0.23, 0.24),
                Terrain::Floor => Color::srgb(0.27, 0.28, 0.29),
                Terrain::Obstacle => Color::srgb(0.24, 0.25, 0.26),
                Terrain::Hole => Color::srgb(0.055, 0.06, 0.065),
            };
            commands.spawn((
                CombatObject,
                DespawnOnExit(GameMode::Combat),
                CombatTile {
                    cell,
                    base_color,
                    terrain,
                },
                Mesh3d(meshes.add(Cuboid::new(TILE * 0.98, 0.12, TILE * 0.98))),
                MeshMaterial3d(materials.add(base_color)),
                Transform::from_translation(position - Vec3::Y * 0.06),
            ));
            commands.spawn((
                CombatObject,
                DespawnOnExit(GameMode::Combat),
                MovementRangeTile(cell),
                Mesh3d(meshes.add(Cuboid::new(TILE * 0.94, 0.025, TILE * 0.94))),
                MeshMaterial3d(movement_range_material.clone()),
                Transform::from_translation(position + Vec3::Y * 0.02),
                Visibility::Hidden,
            ));
            if terrain == Terrain::Obstacle {
                commands.spawn((
                    CombatObject,
                    DespawnOnExit(GameMode::Combat),
                    Mesh3d(meshes.add(Cuboid::new(TILE * 1.01, 0.55, TILE * 1.01))),
                    MeshMaterial3d(materials.add(Color::srgb(0.58, 0.59, 0.59))),
                    Transform::from_translation(position + Vec3::Y * 0.275),
                ));
            }
        }
    }
    let player_cell = map.player_cells()[0];
    turn_order.entities.push(spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Player,
        player_cell,
        100,
        Color::srgb(0.15, 0.38, 0.95),
        equipment.0,
    ));
    for cell in map.enemy_cells().into_iter().take(2) {
        turn_order.entities.push(spawn_unit(
            &mut commands,
            &mut meshes,
            &mut materials,
            Team::Enemy,
            cell,
            20,
            Color::srgb(0.9, 0.06, 0.04),
            Default::default(),
        ));
    }
    let phase_grid = super::hud_layout::HudGrid::content_sized(
        1,
        1,
        0.0,
        0.0,
        UiRect::new(px(8), px(28), px(8), px(8)),
    );
    let (_, phase_content) = super::hud::spawn_hud_window(
        &mut commands,
        "Phase",
        Some(KeyCode::KeyD),
        CombatPhasePanel,
        Vec2::new(352.0, 624.0),
        phase_grid,
        super::hud::HudModuleType::CombatOnly,
        true,
    );
    commands.entity(phase_content).with_children(|grid| {
        grid.spawn((
            CombatPreparation,
            Text::new("Preparation Phase"),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });

    let stats_grid = super::hud_layout::HudGrid::content_sized(
        3,
        1,
        12.0,
        0.0,
        UiRect::new(px(8), px(28), px(8), px(8)),
    );
    let (_, stats_content) = super::hud::spawn_hud_window(
        &mut commands,
        "HP / MP / AP",
        Some(KeyCode::KeyH),
        CombatStats,
        Vec2::new(16.0, 192.0),
        stats_grid,
        super::hud::HudModuleType::CombatOnly,
        true,
    );
    commands.entity(stats_content).with_children(|grid| {
        for stat in [CombatStat::Hp, CombatStat::Mp, CombatStat::Ap] {
            grid.spawn((
                CombatStatCell(stat),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        }
    });

    let timeline_grid = super::hud_layout::HudGrid::content_sized(
        1,
        turn_order.entities.len(),
        0.0,
        4.0,
        UiRect::new(px(8), px(8), px(0), px(2)),
    );
    let (_, timeline_content) = super::hud::spawn_hud_window(
        &mut commands,
        "Timeline",
        Some(KeyCode::KeyT),
        CombatTimeline,
        Vec2::new(16.0, 416.0),
        timeline_grid,
        super::hud::HudModuleType::CombatOnly,
        true,
    );
    commands.entity(timeline_content).with_children(|entries| {
        for unit in turn_order.entities.iter().copied() {
            let item_grid = super::hud_layout::HudGrid::from_tracks(
                vec![GridTrack::flex(1.0), GridTrack::auto()],
                vec![GridTrack::auto(), GridTrack::px(5.0)],
                8.0,
                2.0,
                UiRect::axes(px(6.0), px(2.0)),
            );
            entries
                .spawn((
                    item_grid.node(),
                    BackgroundColor(Color::srgba(0.15, 0.19, 0.24, 0.9)),
                    Pickable::IGNORE,
                ))
                .with_children(|entry| {
                    entry.spawn((
                        TimelineEntryName(unit),
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    entry.spawn((
                        TimelineEntryHealth(unit),
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    entry
                        .spawn((
                            Node {
                                grid_column: GridPlacement::span(2),
                                height: px(5.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.20, 0.08, 0.09)),
                            Pickable::IGNORE,
                        ))
                        .with_children(|track| {
                            track.spawn((
                                TimelineHealthBar(unit),
                                Node {
                                    width: percent(100),
                                    height: percent(100),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.88, 0.12, 0.16)),
                                Pickable::IGNORE,
                            ));
                        });
                });
        }
    });
}

fn spawn_unit(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    team: Team,
    cell: IVec2,
    hp: i32,
    color: Color,
    equipment_modifiers: super::inventory::EquipmentModifiers,
) -> Entity {
    commands
        .spawn((
            CombatObject,
            DespawnOnExit(GameMode::Combat),
            Unit {
                team,
                cell,
                hp: hp + equipment_modifiers.max_hp,
                max_hp: hp + equipment_modifiers.max_hp,
                mp: (if team == Team::Player { 3 } else { 0 }) + equipment_modifiers.mp,
                ap: (if team == Team::Player { 6 } else { 0 }) + equipment_modifiers.ap,
                physical_damage: equipment_modifiers.physical_damage,
                magical_damage: equipment_modifiers.magical_damage,
                equipment_modifiers,
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

fn open_ability_context_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    abilities: Query<(&Hovered, &AbilityIcon)>,
    menus: Query<Entity, With<AbilityInfoContextMenu>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some((_, AbilityIcon(index))) = abilities.iter().next() else {
        return;
    };
    let Some(cursor) = window.and_then(|window| window.cursor_position()) else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    let position = crate::hud_layout::constrain_panel_position(cursor, Vec2::new(112.0, 36.0));
    commands
        .spawn((
            AbilityInfoContextMenu,
            super::hud::HudContextMenu,
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x),
                top: px(position.y),
                width: px(112.0),
                height: px(36.0),
                padding: UiRect::horizontal(px(8.0)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.15, 0.20)),
            BorderColor::all(Color::srgb(0.42, 0.52, 0.64)),
            ZIndex(100),
        ))
        .with_children(|menu| {
            menu.spawn((
                AbilityInfoAction(*index),
                UiButton,
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Info"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn select_ability_info(
    mut commands: Commands,
    book: Res<SpellBook>,
    actions: Query<(&Pressed, &AbilityInfoAction), Changed<Pressed>>,
    menus: Query<Entity, With<AbilityInfoContextMenu>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
) {
    let Some((_, AbilityInfoAction(index))) = actions.iter().next() else {
        return;
    };
    for menu in &menus {
        commands.entity(menu).despawn();
    }
    let Some(spell) = book.0.get(*index) else {
        return;
    };
    let position = window
        .and_then(|window| window.cursor_position())
        .unwrap_or(Vec2::new(400.0, 200.0))
        + Vec2::new(16.0, 16.0);
    let info_rows = 5
        + usize::from(!spell.description.is_empty())
        + usize::from(spell.base_damage > 0)
        + spell.effects.len();
    let grid =
        super::hud_layout::HudGrid::content_sized(1, info_rows, 16.0, 8.0, UiRect::all(px(10.0)));
    let (_, content) = super::hud::spawn_hud_window(
        &mut commands,
        &format!("{} info", spell.name),
        None,
        super::hud::TemporaryHudWindow,
        position,
        grid,
        super::hud::HudModuleType::Both,
        true,
    );
    commands.entity(content).with_children(|body| {
        ability_info_line(body, &spell.name, Color::WHITE);
        if !spell.description.is_empty() {
            ability_info_line(
                body,
                &spell.description,
                super::hud::stat_color(super::hud::StatCategory::Neutral),
            );
        }
        ability_info_line(
            body,
            &format!("AP Cost: {}", spell.ap_cost),
            super::hud::stat_color(super::hud::StatCategory::Ap),
        );
        ability_info_line(
            body,
            &format!("Range: {}", spell.range),
            super::hud::stat_color(super::hud::StatCategory::Neutral),
        );
        ability_info_line(
            body,
            &format!("Target: {:?}", spell.target),
            super::hud::stat_color(super::hud::StatCategory::Neutral),
        );
        ability_info_line(
            body,
            &format!(
                "Area: {}x{}",
                spell.area.first().map_or(0, Vec::len),
                spell.area.len()
            ),
            super::hud::stat_color(super::hud::StatCategory::Neutral),
        );
        if spell.base_damage > 0 {
            let category = match spell.damage_type {
                DamageType::Physical => super::hud::StatCategory::Physical,
                DamageType::Magical => super::hud::StatCategory::Magical,
            };
            ability_info_line(
                body,
                &format!(
                    "{} Damage: {}",
                    match spell.damage_type {
                        DamageType::Physical => "Physical",
                        DamageType::Magical => "Magical",
                    },
                    spell.base_damage
                ),
                super::hud::stat_color(category),
            );
        }
        for effect in &spell.effects {
            let suffix = match effect.stat {
                SpellStat::PhysicalDamage | SpellStat::MagicalDamage => "%",
                _ => "",
            };
            ability_info_line(
                body,
                &format!(
                    "{}: {:+}{suffix}",
                    spell_stat_label(effect.stat),
                    effect.amount
                ),
                spell_stat_color(effect.stat),
            );
        }
    });
}

fn ability_info_line(parent: &mut ChildSpawnerCommands, text: &str, color: Color) {
    parent.spawn((
        Text::new(text.to_owned()),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(color),
        TextLayout::new(Justify::Left, LineBreak::WordBoundary),
    ));
}

fn spell_stat_label(stat: SpellStat) -> &'static str {
    match stat {
        SpellStat::Hp => "HP",
        SpellStat::MaxHp => "Max HP",
        SpellStat::Mp => "MP",
        SpellStat::Ap => "AP",
        SpellStat::PhysicalDamage => "Physical Damage",
        SpellStat::MagicalDamage => "Magical Damage",
    }
}

fn spell_stat_color(stat: SpellStat) -> Color {
    let category = match stat {
        SpellStat::Hp | SpellStat::MaxHp => super::hud::StatCategory::Hp,
        SpellStat::Mp => super::hud::StatCategory::Mp,
        SpellStat::Ap => super::hud::StatCategory::Ap,
        SpellStat::PhysicalDamage => super::hud::StatCategory::Physical,
        SpellStat::MagicalDamage => super::hud::StatCategory::Magical,
    };
    super::hud::stat_color(category)
}

fn combat_input(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    viewport: Res<GameViewport>,
    hud: Res<super::hud::HudState>,
    ability_interactions: Query<(&AbilityIcon, &Pressed), Changed<Pressed>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    book: Res<SpellBook>,
    mut session: ResMut<Session>,
    mut turn_order: ResMut<TurnOrder>,
    mut next: ResMut<NextState<GameMode>>,
    map: Res<CombatMap>,
    mut preparation: ResMut<PreparationPhase>,
    mut units: Query<(Entity, &mut Unit, &mut Transform)>,
) {
    let is_player_turn = turn_order
        .entities
        .get(turn_order.active)
        .is_some_and(|active| turn_order.entities.first() == Some(active));
    if keyboard.just_pressed(KeyCode::Escape) {
        session.spell_selected = None;
    }
    if preparation.0 {
        if keyboard.just_pressed(KeyCode::Space) {
            preparation.0 = false;
            session.spell_selected = None;
            return;
        }
        if mouse.just_pressed(MouseButton::Left)
            && let Some(cursor) = viewport.cursor
            && !hud.contains_cursor(cursor)
        {
            let (camera, transform) = *camera;
            if let Ok(ray) = camera.viewport_to_world(transform, cursor)
                && let Some(point) =
                    ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
            {
                let cell = world_to_cell(point);
                if map.is_player_spawn(cell)
                    && map.walkable(cell)
                    && !units
                        .iter()
                        .any(|(_, unit, _)| unit.team == Team::Enemy && unit.cell == cell)
                    && let Some((entity, _, _)) =
                        units.iter().find(|(_, unit, _)| unit.team == Team::Player)
                    && let Ok((_, mut unit, mut transform)) = units.get_mut(entity)
                {
                    unit.cell = cell;
                    transform.translation = cell_to_world(cell) + Vec3::Y * 0.85;
                }
            }
        }
        return;
    }
    if keyboard.just_pressed(KeyCode::Space) && is_player_turn && !turn_order.entities.is_empty() {
        turn_order.active = (turn_order.active + 1) % turn_order.entities.len();
        session.spell_selected = None;
        return;
    }
    if !is_player_turn {
        return;
    }
    if let Some((AbilityIcon(index), _)) = ability_interactions.iter().next() {
        session.spell_selected = Some(*index);
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
    if hud.contains_cursor(cursor) {
        return;
    }
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
        if distance > spell.range
            || player_ap < spell.ap_cost
            || !map.walkable(cell)
            || !line_clear(&map, player_cell, cell)
        {
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
                || !map.walkable(unit.cell)
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
    if distance == 0 {
        return;
    }
    let occupied = units
        .iter()
        .filter(|(_, unit, _)| unit.hp > 0 && unit.cell != player_cell)
        .map(|(_, unit, _)| unit.cell)
        .collect::<Vec<_>>();
    let Some(path) = shortest_path(&map, player_cell, cell, &occupied) else {
        return;
    };
    let path_cost = path.len() as i32;
    if path_cost > player_mp {
        return;
    }
    if let Ok((_, mut player, mut transform)) = units.get_mut(player_entity) {
        player.mp -= path_cost;
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
    map: Res<CombatMap>,
    preparation: Res<PreparationPhase>,
    book: Res<SpellBook>,
    turn_order: Res<TurnOrder>,
    viewport: Res<GameViewport>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    units: Query<&Unit>,
    mut tiles: Query<(&CombatTile, &MeshMaterial3d<StandardMaterial>)>,
    mut movement_tiles: Query<(&MovementRangeTile, &mut Visibility)>,
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
    let movement_range = hovered_unit
        .and_then(|cell| units.iter().find(|unit| unit.hp > 0 && unit.cell == cell))
        .map(|unit| {
            let occupied = units
                .iter()
                .filter(|other| other.hp > 0 && other.cell != unit.cell)
                .map(|other| other.cell)
                .collect::<Vec<_>>();
            reachable_cells(&map, unit.cell, unit.mp, &occupied)
        })
        .unwrap_or_default();
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
        (player_turn
            && distance <= spell.range
            && enough_ap
            && map.walkable(center)
            && line_clear(&map, origin, center))
        .then(|| area_cells(&spell.area, center))
    });
    let mut path = Vec::new();
    if !preparation.0
        && player_turn
        && session.spell_selected.is_none()
        && let (Some(start), Some(end)) = (player_cell, hovered_cell)
    {
        let occupied = units
            .iter()
            .filter(|unit| unit.hp > 0 && unit.cell != start)
            .map(|unit| unit.cell)
            .collect::<Vec<_>>();
        let player_mp = player_data.map_or(0, |(_, mp, _)| mp);
        if let Some(found) = shortest_path(&map, start, end, &occupied)
            && !found.is_empty()
            && found.len() as i32 <= player_mp
        {
            path = found;
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
    for (tile, mut visibility) in &mut movement_tiles {
        *visibility = if movement_range.contains(&tile.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for (tile, material_handle) in &mut tiles {
        let color = if preparation.0 && map.is_player_spawn(tile.cell) {
            Color::srgba(0.10, 0.42, 0.95, 1.0)
        } else if preparation.0 && map.is_enemy_spawn(tile.cell) {
            Color::srgba(0.78, 0.18, 0.20, 1.0)
        } else if preview_area
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
        } else if tile.terrain == Terrain::Obstacle {
            Color::srgb(0.24, 0.25, 0.26)
        } else if tile.terrain == Terrain::Hole {
            Color::srgb(0.055, 0.06, 0.065)
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

fn shortest_path(
    map: &CombatMap,
    origin: IVec2,
    target: IVec2,
    occupied: &[IVec2],
) -> Option<Vec<IVec2>> {
    use std::collections::{HashMap, VecDeque};
    if !map.walkable(target) || occupied.contains(&target) {
        return None;
    }
    let mut queue = VecDeque::from([origin]);
    let mut previous = HashMap::from([(origin, origin)]);
    while let Some(cell) = queue.pop_front() {
        if cell == target {
            break;
        }
        for neighbor in [
            cell + IVec2::X,
            cell - IVec2::X,
            cell + IVec2::Y,
            cell - IVec2::Y,
        ] {
            if map.walkable(neighbor)
                && !occupied.contains(&neighbor)
                && !previous.contains_key(&neighbor)
            {
                previous.insert(neighbor, cell);
                queue.push_back(neighbor);
            }
        }
    }
    if !previous.contains_key(&target) {
        return None;
    }
    let mut path = Vec::new();
    let mut cell = target;
    while cell != origin {
        path.push(cell);
        cell = previous[&cell];
    }
    path.reverse();
    Some(path)
}

fn reachable_cells(
    map: &CombatMap,
    origin: IVec2,
    movement: i32,
    occupied: &[IVec2],
) -> Vec<IVec2> {
    (0..map.width)
        .flat_map(|x| (0..map.height).map(move |y| IVec2::new(x, y)))
        .filter(|cell| {
            *cell != origin
                && shortest_path(map, origin, *cell, occupied)
                    .is_some_and(|path| path.len() as i32 <= movement)
        })
        .collect()
}

fn line_clear(map: &CombatMap, from: IVec2, to: IVec2) -> bool {
    let steps = (to.x - from.x).abs().max((to.y - from.y).abs());
    (1..steps).all(|step| {
        let t = step as f32 / steps as f32;
        let cell = IVec2::new(
            (from.x as f32 + (to.x - from.x) as f32 * t).round() as i32,
            (from.y as f32 + (to.y - from.y) as f32 * t).round() as i32,
        );
        map.walkable(cell)
    })
}

fn update_combat_preparation(
    preparation: Res<PreparationPhase>,
    mut text: Single<&mut Text, With<CombatPreparation>>,
) {
    text.0 = if preparation.0 {
        "Preparation Phase".to_owned()
    } else {
        "Combat Phase".to_owned()
    };
}

fn update_equipment_stats(
    equipment: Res<super::inventory::CurrentEquipmentModifiers>,
    mut players: Query<&mut Unit>,
) {
    for mut unit in &mut players {
        if unit.team != Team::Player || unit.equipment_modifiers == equipment.0 {
            continue;
        }
        let delta = super::inventory::EquipmentModifiers {
            max_hp: equipment.0.max_hp - unit.equipment_modifiers.max_hp,
            mp: equipment.0.mp - unit.equipment_modifiers.mp,
            ap: equipment.0.ap - unit.equipment_modifiers.ap,
            physical_damage: equipment.0.physical_damage - unit.equipment_modifiers.physical_damage,
            magical_damage: equipment.0.magical_damage - unit.equipment_modifiers.magical_damage,
        };
        unit.max_hp = (unit.max_hp + delta.max_hp).max(1);
        unit.hp = (unit.hp + delta.max_hp).clamp(0, unit.max_hp);
        unit.mp = (unit.mp + delta.mp).max(0);
        unit.ap = (unit.ap + delta.ap).max(0);
        unit.physical_damage += delta.physical_damage;
        unit.magical_damage += delta.magical_damage;
        unit.equipment_modifiers = equipment.0;
    }
}

fn update_combat_stats(
    mut stats: Query<(&mut Text, &mut TextColor, &CombatStatCell)>,
    units: Query<&Unit>,
) {
    let player = units.iter().find(|unit| unit.team == Team::Player);
    for (mut text, mut color, CombatStatCell(stat)) in &mut stats {
        let (value, category) = match stat {
            CombatStat::Hp => (
                format!(
                    "HP {}/{}",
                    player.map_or(0, |unit| unit.hp.max(0)),
                    player.map_or(0, |unit| unit.max_hp)
                ),
                super::hud::StatCategory::Hp,
            ),
            CombatStat::Mp => (
                format!("MP {}", player.map_or(0, |unit| unit.mp)),
                super::hud::StatCategory::Mp,
            ),
            CombatStat::Ap => (
                format!("AP {}", player.map_or(0, |unit| unit.ap)),
                super::hud::StatCategory::Ap,
            ),
        };
        text.0 = value;
        color.0 = super::hud::stat_color(category);
    }
}

fn ability_color(spell: &Spell) -> Color {
    if spell.base_damage == 0 {
        Color::srgb(0.35, 0.37, 0.40)
    } else {
        match spell.damage_type {
            DamageType::Magical => Color::srgb(0.48, 0.22, 0.72),
            DamageType::Physical => Color::srgb(0.85, 0.38, 0.12),
        }
    }
}

fn update_combat_spells(session: Res<Session>, mut icons: Query<(&mut BorderColor, &AbilityIcon)>) {
    for (mut border, AbilityIcon(index)) in &mut icons {
        let color = if session.spell_selected == Some(*index) {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.35)
        };
        *border = BorderColor::all(color);
    }
}

fn update_combat_timeline(
    turn_order: Res<TurnOrder>,
    units: Query<&Unit>,
    mut names: Query<(&mut Text, &TimelineEntryName), Without<TimelineEntryHealth>>,
    mut health_numbers: Query<(&mut Text, &TimelineEntryHealth), Without<TimelineEntryName>>,
    mut health_bars: Query<(&mut Node, &TimelineHealthBar)>,
) {
    for (mut node, TimelineHealthBar(entity)) in &mut health_bars {
        let Ok(unit) = units.get(*entity) else {
            node.width = percent(0);
            continue;
        };
        let max_hp = unit.max_hp.max(0);
        let ratio = if max_hp == 0 {
            0.0
        } else {
            unit.hp.clamp(0, max_hp) as f32 / max_hp as f32
        };
        node.width = percent(ratio * 100.0);
    }

    for (mut text, TimelineEntryName(entity)) in &mut names {
        let Ok(unit) = units.get(*entity) else {
            text.0 = "Unavailable".to_owned();
            continue;
        };
        let Some(index) = turn_order
            .entities
            .iter()
            .position(|current| current == entity)
        else {
            continue;
        };
        let label = if index == 0 {
            "Player"
        } else if unit.team == Team::Enemy {
            if index == 1 { "Enemy 1" } else { "Enemy 2" }
        } else {
            "Entity"
        };
        let status = if unit.team == Team::Enemy && unit.hp <= 0 {
            " (DEAD)"
        } else {
            ""
        };
        let active = if index == turn_order.active && unit.hp > 0 {
            "> "
        } else {
            ""
        };
        text.0 = format!("{active}{label}{status}");
    }

    for (mut text, TimelineEntryHealth(entity)) in &mut health_numbers {
        let Ok(unit) = units.get(*entity) else {
            text.0 = "—".to_owned();
            continue;
        };
        let current_hp = unit.hp.clamp(0, unit.max_hp.max(0));
        let max_hp = unit.max_hp.max(0);
        text.0 = format!("{current_hp}/{max_hp}");
    }
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
            equipment_modifiers: Default::default(),
        }
    }

    #[test]
    fn saved_map_has_valid_deployment_cells_and_obstacles_and_holes_block_paths() {
        let map = CombatMap::load();
        assert!(map.player_cells().iter().all(|cell| map.walkable(*cell)));
        assert!(map.enemy_cells().iter().all(|cell| map.walkable(*cell)));
        assert!(!map.walkable(IVec2::new(0, 2)));
        assert!(!map.walkable(IVec2::new(4, 1)));
        assert!(shortest_path(&map, IVec2::new(2, 3), IVec2::new(1, 3), &[]).is_none());
        assert!(!line_clear(&map, IVec2::new(3, 1), IVec2::new(1, 1)));
    }

    #[test]
    fn movement_range_contains_only_unoccupied_cells_within_mp() {
        let origin = IVec2::new(2, 2);
        let occupied = [IVec2::new(3, 2)];
        let map = CombatMap::load();
        let cells = reachable_cells(&map, origin, 2, &occupied);

        assert!(!cells.contains(&origin));
        assert!(!cells.contains(&occupied[0]));
        assert!(cells.contains(&IVec2::new(2, 4)));
        assert!(!cells.contains(&IVec2::new(5, 2)));
        assert!(
            cells
                .iter()
                .all(|cell| { (cell.x - origin.x).abs() + (cell.y - origin.y).abs() <= 2 })
        );
    }

    #[test]
    fn configured_spells_deserialize_and_have_valid_costs_and_distances() {
        let spells: Vec<Spell> = serde_json::from_str(include_str!("../assets/spells.json"))
            .expect("the example spell configuration should deserialize");
        assert!(!spells.is_empty());
        assert!(spells.iter().all(|spell| !spell.description.is_empty()));
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

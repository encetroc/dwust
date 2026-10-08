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

#[derive(Resource, Default)]
struct Session {
    mp: u8,
    ap: u8,
    spell_selected: bool,
    camera_transform: Option<Transform>,
}

#[derive(Component)]
struct CombatHud;

pub(super) struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .add_systems(Startup, spawn_world_enemies)
            .add_systems(OnEnter(GameMode::Combat), enter_combat)
            .add_systems(OnExit(GameMode::Combat), exit_combat)
            .add_systems(Update, start_combat.run_if(in_state(GameMode::World)))
            .add_systems(
                Update,
                (combat_input, update_combat_hud, finish_if_won)
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
    mouse: Res<ButtonInput<MouseButton>>,
    viewport: Res<GameViewport>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    enemies: Query<&GlobalTransform, With<WorldEnemy>>,
    mut next: ResMut<NextState<GameMode>>,
    mut player: Single<&mut Player>,
) {
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
    if enemies
        .iter()
        .any(|enemy| ray_sphere_hit(ray, enemy.translation(), 0.95))
    {
        player.destination = None;
        player.path.clear();
        player.travel = None;
        next.set(GameMode::Combat);
    }
}

pub(super) fn ray_hits_enemy(
    ray: Ray3d,
    enemies: &Query<&GlobalTransform, With<WorldEnemy>>,
) -> bool {
    enemies
        .iter()
        .any(|enemy| ray_sphere_hit(ray, enemy.translation(), 0.95))
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

    let floor_materials = [
        materials.add(Color::srgb(0.30, 0.34, 0.25)),
        materials.add(Color::srgb(0.39, 0.42, 0.31)),
    ];
    for x in 0..GRID_W {
        for z in 0..GRID_H {
            let cell = IVec2::new(x, z);
            let position = cell_to_world(cell);
            commands.spawn((
                CombatObject,
                DespawnOnExit(GameMode::Combat),
                Mesh3d(meshes.add(Cuboid::new(TILE * 0.98, 0.12, TILE * 0.98))),
                MeshMaterial3d(floor_materials[((x + z) & 1) as usize].clone()),
                Transform::from_translation(position - Vec3::Y * 0.06),
            ));
        }
    }
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Player,
        IVec2::new(1, 2),
        100,
        Color::srgb(0.15, 0.38, 0.95),
    );
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Enemy,
        IVec2::new(6, 1),
        20,
        Color::srgb(0.9, 0.06, 0.04),
    );
    spawn_unit(
        &mut commands,
        &mut meshes,
        &mut materials,
        Team::Enemy,
        IVec2::new(6, 4),
        20,
        Color::srgb(0.9, 0.06, 0.04),
    );
    commands.spawn((
        CombatObject,
        DespawnOnExit(GameMode::Combat),
        CombatHud,
        Pickable::IGNORE,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(19.0),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.04, 0.05, 0.06, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
    ));
}

fn spawn_unit(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    team: Team,
    cell: IVec2,
    hp: i32,
    color: Color,
) {
    commands.spawn((
        CombatObject,
        DespawnOnExit(GameMode::Combat),
        Unit { team, cell, hp },
        Mesh3d(meshes.add(Sphere::new(0.72).mesh().uv(24, 16))),
        MeshMaterial3d(materials.add(color)),
        Transform::from_translation(cell_to_world(cell) + Vec3::Y * 0.85),
    ));
}

fn exit_combat(
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
    mut next: ResMut<NextState<GameMode>>,
    mut units: Query<(&mut Unit, &mut Transform)>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        session.spell_selected = true;
    }
    if keyboard.just_pressed(KeyCode::Enter) {
        session.mp = 3;
        session.ap = 6;
        session.spell_selected = false;
        return; // Enemies have no AI yet: their turn is skipped.
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
        .find_map(|(unit, _)| (unit.team == Team::Player).then_some(unit.cell));
    let Some(player_cell) = player_cell else {
        return;
    };
    let distance = (cell.x - player_cell.x).abs() + (cell.y - player_cell.y).abs();
    let target = units
        .iter()
        .position(|(unit, _)| unit.team == Team::Enemy && unit.hp > 0 && unit.cell == cell);
    if session.spell_selected {
        if let Some(target) = target
            && distance <= SPELL_RANGE
            && session.ap >= SPELL_AP
        {
            session.ap -= SPELL_AP;
            session.spell_selected = false;
            if let Some((mut unit, mut transform)) = units.iter_mut().nth(target) {
                unit.hp -= SPELL_DAMAGE;
                if unit.hp <= 0 {
                    transform.scale = Vec3::splat(0.001);
                }
            }
            if units
                .iter()
                .all(|(unit, _)| unit.team != Team::Enemy || unit.hp <= 0)
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
        .any(|(unit, _)| unit.team == Team::Enemy && unit.cell == cell && unit.hp > 0)
    {
        return;
    }
    session.mp -= distance as u8;
    if let Some((mut unit, mut transform)) =
        units.iter_mut().find(|(unit, _)| unit.team == Team::Player)
    {
        unit.cell = cell;
        transform.translation = cell_to_world(cell) + Vec3::Y * 0.85;
    }
}

fn update_combat_hud(
    session: Res<Session>,
    mut hud: Single<&mut Text, With<CombatHud>>,
    units: Query<&Unit>,
) {
    let enemy_info = units
        .iter()
        .filter(|unit| unit.team == Team::Enemy && unit.hp > 0)
        .count();
    hud.0 = format!(
        "Your turn  |  MP {}  |  AP {}  |  Enemies {}\n[1] Select spell: 10 damage, 3 PO, 3 AP  |  Click a tile to move  |  Enter: end turn",
        session.mp, session.ap, enemy_info
    );
}

fn finish_if_won(units: Query<&Unit, With<CombatObject>>, mut next: ResMut<NextState<GameMode>>) {
    if units
        .iter()
        .all(|unit| unit.team != Team::Enemy || unit.hp <= 0)
    {
        next.set(GameMode::World);
    }
}

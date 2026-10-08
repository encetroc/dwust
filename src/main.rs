use bevy::{
    camera::ScalingMode, prelude::*, render::render_resource::TextureFormat, window::PrimaryWindow,
};

mod buildings;
mod combat;
mod config;
mod headless;
use config::*;

#[derive(Resource, Default)]
struct ScreenAnimation(Option<ScreenTween>);

struct ScreenTween {
    elapsed: f32,
    duration: f32,
    camera_start: Vec3,
    camera_end: Vec3,
    ground_start: Vec3,
    ground_end: Vec3,
    player_start: Vec3,
    player_end: Vec3,
}

fn transition_progress(elapsed: f32, duration: f32) -> f32 {
    let t = (elapsed / duration).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Resource, Default)]
struct WorldScreen {
    coordinates: IVec2,
    center: Vec3,
}

#[derive(Component)]
struct Ground;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct ScreenPreview(ScreenDirection);

#[derive(Component, Clone, Copy)]
enum ScreenDirection {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl ScreenDirection {
    fn grid_offset(self) -> IVec2 {
        match self {
            Self::Up => IVec2::new(0, -1),
            Self::Down => IVec2::new(0, 1),
            Self::Left => IVec2::new(-1, 0),
            Self::Right => IVec2::new(1, 0),
            Self::UpLeft => IVec2::new(-1, -1),
            Self::UpRight => IVec2::new(1, -1),
            Self::DownLeft => IVec2::new(-1, 1),
            Self::DownRight => IVec2::new(1, 1),
        }
    }
}

#[derive(Component, Clone, Copy)]
enum OverlayAction {
    Travel(ScreenDirection),
}

#[derive(Message)]
struct OverlayInput(OverlayAction);

#[derive(Resource, Default)]
struct OverlayPointer {
    hovered: Option<Entity>,
    overlay_pressed: bool,
}

// The same input seam supports a real window and windowless verification.
#[derive(Resource)]
struct GameViewport {
    size: Vec2,
    cursor: Option<Vec2>,
}

impl Default for GameViewport {
    fn default() -> Self {
        Self {
            size: Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
            cursor: None,
        }
    }
}

fn sync_viewport(
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut viewport: ResMut<GameViewport>,
) {
    if let Some(window) = window {
        viewport.size = window.size();
        viewport.cursor = window.cursor_position();
    }
}

// Screen bounds are aligned with the camera, not the isometric world axes.
fn screen_half_extents(size: Vec2) -> Vec2 {
    Vec2::new(VIEW_HEIGHT * size.x / size.y.max(1.0) / 2.0, VIEW_HEIGHT)
}

fn clamp_to_screen(position: Vec3, center: Vec3, half_extents: Vec2) -> Vec3 {
    let local = position - center;
    let bounds = (half_extents - SCREEN_MARGIN).max(Vec2::ZERO);
    center
        + SCREEN_RIGHT * local.dot(SCREEN_RIGHT).clamp(-bounds.x, bounds.x)
        + SCREEN_UP * local.dot(SCREEN_UP).clamp(-bounds.y, bounds.y)
        + Vec3::Y * (PLAYER_HEIGHT / 2.0)
}

fn screen_transition(
    position: Vec3,
    center: Vec3,
    half_extents: Vec2,
    direction: ScreenDirection,
) -> (Vec3, Vec3) {
    let grid = direction.grid_offset();
    let offset = SCREEN_RIGHT * (grid.x as f32 * half_extents.x * 2.0)
        - SCREEN_UP * (grid.y as f32 * half_extents.y * 2.0);
    let next_center = center + offset;
    let local = clamp_to_screen(position, center, half_extents) - center;
    let bounds = (half_extents - SCREEN_MARGIN).max(Vec2::ZERO);
    let mut horizontal = local.dot(SCREEN_RIGHT);
    let mut vertical = local.dot(SCREEN_UP);
    let offset = direction.grid_offset();
    if offset.x < 0 {
        horizontal = bounds.x;
    } else if offset.x > 0 {
        horizontal = -bounds.x;
    }
    if offset.y < 0 {
        vertical = -bounds.y;
    } else if offset.y > 0 {
        vertical = bounds.y;
    }
    (
        next_center,
        next_center
            + SCREEN_RIGHT * horizontal
            + SCREEN_UP * vertical
            + Vec3::Y * (PLAYER_HEIGHT / 2.0),
    )
}

#[derive(Component, Default)]
struct Player {
    destination: Option<Vec3>,
    path: Vec<Vec3>,
    travel: Option<ScreenDirection>,
    approach_enemy: Option<Entity>,
}

fn travel_destination(
    position: Vec3,
    center: Vec3,
    half_extents: Vec2,
    direction: ScreenDirection,
) -> Vec3 {
    let bounds = (half_extents - SCREEN_MARGIN).max(Vec2::ZERO);
    let offset = direction.grid_offset();
    let local = position - center;
    let mut horizontal = local.dot(SCREEN_RIGHT).clamp(-bounds.x, bounds.x);
    let mut vertical = local.dot(SCREEN_UP).clamp(-bounds.y, bounds.y);
    if offset.x != 0 {
        horizontal = offset.x as f32 * bounds.x;
    }
    if offset.y != 0 {
        vertical = -(offset.y as f32) * bounds.y;
    }
    center + SCREEN_RIGHT * horizontal + SCREEN_UP * vertical + Vec3::Y * (PLAYER_HEIGHT / 2.0)
}

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<combat::GameMode>()
            .add_plugins(combat::CombatPlugin)
            .init_resource::<MovementSettings>()
            .init_resource::<ScreenAnimationSettings>()
            .init_resource::<ScreenAnimation>()
            .init_resource::<WorldScreen>()
            .init_resource::<GameViewport>()
            .init_resource::<OverlayPointer>()
            .add_message::<OverlayInput>()
            .add_systems(Startup, (setup, setup_overlay, buildings::spawn_buildings))
            .add_systems(PreUpdate, sync_viewport)
            .add_systems(
                Update,
                (
                    handle_overlay_and_travel,
                    set_destination,
                    move_player,
                    finish_travel,
                    animate_screen,
                    update_screen_colors,
                    update_status,
                )
                    .chain()
                    .run_if(in_state(combat::GameMode::World)),
            );
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--verify-headless") {
        headless::run(args.next().expect("provide a fresh output directory"));
        return;
    }
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dwust".into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Ground,
        combat::WorldObject,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(WORLD_SIZE, WORLD_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: screen_color(IVec2::ZERO),
            unlit: true,
            ..default()
        })),
    ));

    commands.spawn((
        Player::default(),
        combat::WorldObject,
        Mesh3d(meshes.add(Capsule3d::new(
            PLAYER_RADIUS,
            PLAYER_HEIGHT - 2.0 * PLAYER_RADIUS,
        ))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.45, 0.85),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, PLAYER_HEIGHT / 2.0, 0.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 45-degree azimuth and 30-degree elevation give Dofus-like 2:1 diamonds.
    // A fixed orthographic camera keeps tiles the same size at every distance.
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(40.0, 40.0 * (2.0_f32 / 3.0).sqrt(), 40.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn screen_color(coordinates: IVec2) -> Color {
    // A repeating 3x3 palette guarantees each screen and its eight neighbors differ.
    const PALETTE: [Color; 9] = [
        Color::srgb(0.85, 0.20, 0.18), // coral red
        Color::srgb(0.16, 0.48, 0.88), // blue
        Color::srgb(0.22, 0.68, 0.34), // green
        Color::srgb(0.95, 0.72, 0.16), // gold
        Color::srgb(0.58, 0.30, 0.78), // purple
        Color::srgb(0.12, 0.72, 0.76), // cyan
        Color::srgb(0.94, 0.43, 0.14), // orange
        Color::srgb(0.88, 0.34, 0.60), // pink
        Color::srgb(0.40, 0.56, 0.24), // olive
    ];
    let column = coordinates.x.rem_euclid(3) as usize;
    let row = coordinates.y.rem_euclid(3) as usize;
    PALETTE[row * 3 + column]
}

fn setup_overlay(mut commands: Commands) {
    commands.spawn((
        StatusText,
        combat::WorldUi,
        Pickable::IGNORE,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::WHITE),
        BackgroundColor(Color::srgba(0.05, 0.07, 0.1, 0.85)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(8)),
            ..default()
        },
    ));

    // Match the reference: four edge strips plus four corner previews around the current screen.
    for (direction, node) in [
        (
            ScreenDirection::Up,
            Node {
                top: percent(0),
                left: percent(7),
                width: percent(86),
                height: percent(10),
                ..default()
            },
        ),
        (
            ScreenDirection::Down,
            Node {
                bottom: percent(0),
                left: percent(7),
                width: percent(86),
                height: percent(10),
                ..default()
            },
        ),
        (
            ScreenDirection::Left,
            Node {
                left: percent(0),
                top: percent(10),
                width: percent(7),
                height: percent(80),
                ..default()
            },
        ),
        (
            ScreenDirection::Right,
            Node {
                right: percent(0),
                top: percent(10),
                width: percent(7),
                height: percent(80),
                ..default()
            },
        ),
        (
            ScreenDirection::UpLeft,
            Node {
                top: percent(0),
                left: percent(0),
                width: percent(7),
                height: percent(10),
                ..default()
            },
        ),
        (
            ScreenDirection::UpRight,
            Node {
                top: percent(0),
                right: percent(0),
                width: percent(7),
                height: percent(10),
                ..default()
            },
        ),
        (
            ScreenDirection::DownLeft,
            Node {
                bottom: percent(0),
                left: percent(0),
                width: percent(7),
                height: percent(10),
                ..default()
            },
        ),
        (
            ScreenDirection::DownRight,
            Node {
                bottom: percent(0),
                right: percent(0),
                width: percent(7),
                height: percent(10),
                ..default()
            },
        ),
    ] {
        let mut node = node;
        node.position_type = PositionType::Absolute;
        commands
            .spawn((
                ScreenPreview(direction),
                OverlayAction::Travel(direction),
                combat::WorldUi,
                Pickable::default(),
                node,
                BackgroundColor(screen_color(direction.grid_offset())),
            ))
            .observe(overlay_over)
            .observe(overlay_out)
            .observe(overlay_press);
    }
}

fn overlay_over(
    event: On<Pointer<Over>>,
    mut pointer: ResMut<OverlayPointer>,
    mut colors: Query<&mut BackgroundColor>,
) {
    pointer.hovered = Some(event.entity);
    if let Ok(mut color) = colors.get_mut(event.entity) {
        color.0 = Color::srgb(0.2, 0.3, 0.4);
    }
}

fn overlay_out(
    event: On<Pointer<Out>>,
    mut pointer: ResMut<OverlayPointer>,
    screen: Res<WorldScreen>,
    mut previews: Query<(&ScreenPreview, &mut BackgroundColor)>,
) {
    if pointer.hovered == Some(event.entity) {
        pointer.hovered = None;
    }
    if let Ok((preview, mut color)) = previews.get_mut(event.entity) {
        color.0 = screen_color(screen.coordinates + preview.0.grid_offset());
    }
}

fn overlay_press(
    mut event: On<Pointer<Press>>,
    actions: Query<&OverlayAction>,
    mut inputs: MessageWriter<OverlayInput>,
    mut pointer: ResMut<OverlayPointer>,
) {
    if event.button == PointerButton::Primary
        && let Ok(action) = actions.get(event.entity)
    {
        inputs.write(OverlayInput(*action));
        pointer.overlay_pressed = true;
        event.propagate(false);
    }
}

// Explicit disjoint queries keep Bevy's mutable transform access safe.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn handle_overlay_and_travel(
    keyboard: Res<ButtonInput<KeyCode>>,
    viewport: Res<GameViewport>,
    mut inputs: MessageReader<OverlayInput>,
    animation: Res<ScreenAnimation>,
    screen: Res<WorldScreen>,
    mut player: Single<(&Transform, &mut Player)>,
) {
    let mut travel = None;
    for input in inputs.read() {
        let OverlayAction::Travel(direction) = input.0;
        travel = Some(direction);
    }
    if animation.0.is_some() {
        return;
    }
    for (key, direction) in [
        (KeyCode::ArrowUp, ScreenDirection::Up),
        (KeyCode::ArrowDown, ScreenDirection::Down),
        (KeyCode::ArrowLeft, ScreenDirection::Left),
        (KeyCode::ArrowRight, ScreenDirection::Right),
    ] {
        if keyboard.just_pressed(key) {
            travel = Some(direction);
            break;
        }
    }
    if let Some(direction) = travel {
        let (transform, player) = &mut *player;
        player.approach_enemy = None;
        player.destination = Some(travel_destination(
            transform.translation,
            screen.center,
            screen_half_extents(viewport.size),
            direction,
        ));
        player.path.clear();
        player.travel = Some(direction);
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn finish_travel(
    viewport: Res<GameViewport>,
    settings: Res<ScreenAnimationSettings>,
    mut animation: ResMut<ScreenAnimation>,
    mut screen: ResMut<WorldScreen>,
    mut player: Single<(&mut Transform, &mut Player), (Without<Camera3d>, Without<Ground>)>,
    mut camera: Single<&mut Transform, (With<Camera3d>, Without<Ground>, Without<Player>)>,
    mut ground: Single<&mut Transform, (With<Ground>, Without<Player>, Without<Camera3d>)>,
) {
    let (transform, player) = &mut *player;
    if player.destination.is_none()
        && let Some(direction) = player.travel.take()
    {
        let (center, position) = screen_transition(
            transform.translation,
            screen.center,
            screen_half_extents(viewport.size),
            direction,
        );
        let camera_end = camera.translation + center - screen.center;
        if settings.enabled && settings.duration.is_finite() && settings.duration > 0.0 {
            animation.0 = Some(ScreenTween {
                elapsed: 0.0,
                duration: settings.duration,
                camera_start: camera.translation,
                camera_end,
                ground_start: ground.translation,
                ground_end: center,
                player_start: transform.translation,
                player_end: position,
            });
        } else {
            camera.translation = camera_end;
            ground.translation = center;
            transform.translation = position;
        }
        player.destination = None;
        player.path.clear();
        screen.center = center;
        screen.coordinates += direction.grid_offset();
    }
}

#[allow(clippy::type_complexity)]
fn animate_screen(
    time: Res<Time>,
    mut animation: ResMut<ScreenAnimation>,
    mut player: Single<&mut Transform, (With<Player>, Without<Camera3d>, Without<Ground>)>,
    mut camera: Single<&mut Transform, (With<Camera3d>, Without<Ground>, Without<Player>)>,
    mut ground: Single<&mut Transform, (With<Ground>, Without<Player>, Without<Camera3d>)>,
) {
    let Some(tween) = animation.0.as_mut() else {
        return;
    };
    tween.elapsed += time.delta_secs();
    let progress = transition_progress(tween.elapsed, tween.duration);
    player.translation = tween.player_start.lerp(tween.player_end, progress);
    camera.translation = tween.camera_start.lerp(tween.camera_end, progress);
    ground.translation = tween.ground_start.lerp(tween.ground_end, progress);
    if tween.elapsed >= tween.duration {
        animation.0 = None;
    }
}

fn update_screen_colors(
    screen: Res<WorldScreen>,
    ground: Single<&MeshMaterial3d<StandardMaterial>, With<Ground>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut previews: Query<(&ScreenPreview, &mut BackgroundColor)>,
) {
    if screen.is_changed()
        && let Some(mut material) = materials.get_mut(&ground.0)
    {
        material.base_color = screen_color(screen.coordinates);
    }
    for (preview, mut color) in &mut previews {
        color.0 = screen_color(screen.coordinates + preview.0.grid_offset());
    }
}

fn update_status(screen: Res<WorldScreen>, mut text: Single<&mut Text, With<StatusText>>) {
    if screen.is_changed() {
        text.0 = format!("[{}, {}]", screen.coordinates.x, screen.coordinates.y);
    }
}

fn set_destination(
    mouse: Res<ButtonInput<MouseButton>>,
    mut pointer: ResMut<OverlayPointer>,
    enemies: Query<(Entity, &GlobalTransform), With<combat::WorldEnemy>>,
    animation: Res<ScreenAnimation>,
    screen: Res<WorldScreen>,
    viewport: Res<GameViewport>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut player: Single<&mut Player>,
) {
    if pointer.overlay_pressed {
        pointer.overlay_pressed = false;
        return;
    }
    if animation.0.is_some() || !mouse.just_pressed(MouseButton::Left) || pointer.hovered.is_some()
    {
        return;
    }
    let (camera, camera_transform) = *camera;
    if let Some(cursor) = viewport.cursor
        && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor)
    {
        if let Some((enemy, position)) = combat::ray_hits_enemy(ray, &enemies) {
            player.travel = None;
            player.approach_enemy = Some(enemy);
            player.path.clear();
            player.destination = Some(Vec3::new(position.x, PLAYER_HEIGHT / 2.0, position.z));
        } else if let Some(point) =
            ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
        {
            player.travel = None;
            player.approach_enemy = None;
            player.path.clear();
            player.destination = Some(clamp_to_screen(
                point,
                screen.center,
                screen_half_extents(viewport.size),
            ));
        }
    }
}

#[allow(clippy::type_complexity)]
fn move_player(
    time: Res<Time>,
    settings: Res<MovementSettings>,
    buildings: Query<
        (&buildings::Building, &Transform),
        (With<buildings::Building>, Without<Player>),
    >,
    mut player: Single<(&mut Transform, &mut Player)>,
) {
    let (transform, player) = &mut *player;
    let Some(destination) = player.destination else {
        return;
    };
    if player.path.is_empty() {
        let obstacles = buildings
            .iter()
            .map(|(building, transform)| Obstacle {
                center: Vec2::new(transform.translation.x, transform.translation.z),
                half_size: Vec2::new(building.0.x / 2.0, building.0.z / 2.0)
                    + Vec2::splat(PLAYER_RADIUS + 0.15),
            })
            .collect::<Vec<_>>();
        player.path = find_path(transform.translation, destination, &obstacles);
    }
    let Some(waypoint) = player.path.first().copied() else {
        player.destination = None;
        player.travel = None;
        return;
    };
    transform.translation = step_toward(
        transform.translation,
        waypoint,
        settings.speed * time.delta_secs(),
    );
    if transform.translation.distance_squared(waypoint) < 0.0001 {
        transform.translation = waypoint;
        player.path.remove(0);
        if player.path.is_empty() {
            player.destination = None;
        }
    }
}

#[derive(Clone, Copy)]
struct Obstacle {
    center: Vec2,
    half_size: Vec2,
}

impl Obstacle {
    fn min(self) -> Vec2 {
        self.center - self.half_size
    }

    fn max(self) -> Vec2 {
        self.center + self.half_size
    }

    fn contains(self, point: Vec2) -> bool {
        let min = self.min();
        let max = self.max();
        point.x > min.x && point.x < max.x && point.y > min.y && point.y < max.y
    }

    fn corners(self, height: f32) -> [Vec3; 4] {
        let min = self.min();
        let max = self.max();
        [
            Vec3::new(min.x, height, min.y),
            Vec3::new(min.x, height, max.y),
            Vec3::new(max.x, height, min.y),
            Vec3::new(max.x, height, max.y),
        ]
    }
}

fn segment_hits_obstacle(start: Vec2, end: Vec2, obstacle: Obstacle) -> bool {
    // Shrink very slightly so routes may safely follow the expanded perimeter.
    let min = obstacle.min() + Vec2::splat(0.001);
    let max = obstacle.max() - Vec2::splat(0.001);
    let delta = end - start;
    let mut enter: f32 = 0.0;
    let mut exit: f32 = 1.0;
    for (origin, direction, low, high) in [
        (start.x, delta.x, min.x, max.x),
        (start.y, delta.y, min.y, max.y),
    ] {
        if direction.abs() < f32::EPSILON {
            if origin <= low || origin >= high {
                return false;
            }
            continue;
        }
        let a = (low - origin) / direction;
        let b = (high - origin) / direction;
        enter = enter.max(a.min(b));
        exit = exit.min(a.max(b));
        if enter >= exit {
            return false;
        }
    }
    enter < 1.0 && exit > 0.0 && enter < exit
}

fn find_path(start: Vec3, destination: Vec3, obstacles: &[Obstacle]) -> Vec<Vec3> {
    let mut end_2d = Vec2::new(destination.x, destination.z);
    if let Some(obstacle) = obstacles.iter().find(|obstacle| obstacle.contains(end_2d)) {
        end_2d = obstacle
            .corners(destination.y)
            .into_iter()
            .filter(|corner| {
                !obstacles
                    .iter()
                    .any(|other| other.contains(Vec2::new(corner.x, corner.z)))
            })
            .min_by(|a, b| {
                Vec2::new(a.x, a.z)
                    .distance_squared(end_2d)
                    .total_cmp(&Vec2::new(b.x, b.z).distance_squared(end_2d))
            })
            .map(|point| Vec2::new(point.x, point.z))
            .unwrap_or(end_2d);
    }

    let mut nodes = vec![start, Vec3::new(end_2d.x, destination.y, end_2d.y)];
    for obstacle in obstacles {
        nodes.extend(obstacle.corners(start.y).into_iter().filter(|corner| {
            !obstacles
                .iter()
                .any(|other| other.contains(Vec2::new(corner.x, corner.z)))
        }));
    }
    let count = nodes.len();
    let mut distances = vec![f32::INFINITY; count];
    let mut previous = vec![None; count];
    let mut visited = vec![false; count];
    distances[0] = 0.0;
    for _ in 0..count {
        let Some(current) = (0..count)
            .filter(|&index| !visited[index])
            .min_by(|&a, &b| distances[a].total_cmp(&distances[b]))
        else {
            break;
        };
        if !distances[current].is_finite() || current == 1 {
            break;
        }
        visited[current] = true;
        for next in 0..count {
            if visited[next]
                || obstacles.iter().any(|obstacle| {
                    segment_hits_obstacle(
                        Vec2::new(nodes[current].x, nodes[current].z),
                        Vec2::new(nodes[next].x, nodes[next].z),
                        *obstacle,
                    )
                })
            {
                continue;
            }
            let distance = distances[current] + nodes[current].distance(nodes[next]);
            if distance < distances[next] {
                distances[next] = distance;
                previous[next] = Some(current);
            }
        }
    }
    if !distances[1].is_finite() {
        return Vec::new();
    }
    let mut path = vec![nodes[1]];
    let mut current = 1;
    while let Some(parent) = previous[current] {
        if parent == 0 {
            break;
        }
        path.push(nodes[parent]);
        current = parent;
    }
    path.reverse();
    path
}

fn step_toward(position: Vec3, destination: Vec3, max_distance: f32) -> Vec3 {
    let offset = destination - position;
    let distance = offset.length();
    if distance <= max_distance {
        destination
    } else {
        position + offset / distance * max_distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_configuration_accepts_only_supported_finite_speeds() {
        assert_eq!(configured_speed(Some("24")), 24.0);
        for value in [
            None,
            Some("bad"),
            Some("NaN"),
            Some("inf"),
            Some("0"),
            Some("-5"),
            Some("61"),
        ] {
            assert_eq!(configured_speed(value), DEFAULT_MOVE_SPEED);
        }
        assert!(configured_speed(None) > 6.0);
    }

    #[test]
    fn transitions_enter_the_opposite_edge_and_preserve_the_other_axis() {
        let extents = Vec2::new(28.0, 32.0);
        let center = Vec3::new(120.0, 0.0, -80.0);
        let start = center + SCREEN_RIGHT * 5.0 + SCREEN_UP * 7.0 + Vec3::Y * (PLAYER_HEIGHT / 2.0);
        for (direction, opposite, expected) in [
            (
                ScreenDirection::Up,
                ScreenDirection::Down,
                Vec2::new(5.0, -22.0),
            ),
            (
                ScreenDirection::Down,
                ScreenDirection::Up,
                Vec2::new(5.0, 22.0),
            ),
            (
                ScreenDirection::Left,
                ScreenDirection::Right,
                Vec2::new(24.0, 7.0),
            ),
            (
                ScreenDirection::Right,
                ScreenDirection::Left,
                Vec2::new(-24.0, 7.0),
            ),
        ] {
            let (next_center, entry) = screen_transition(start, center, extents, direction);
            let local = entry - next_center;
            assert!((local.dot(SCREEN_RIGHT) - expected.x).abs() < 0.0001);
            assert!((local.dot(SCREEN_UP) - expected.y).abs() < 0.0001);
            assert_eq!(entry.y, PLAYER_HEIGHT / 2.0);
            let (returned_center, _) = screen_transition(entry, next_center, extents, opposite);
            assert!(returned_center.distance(center) < 0.0001);
            assert!(clamp_to_screen(entry, next_center, extents).distance(entry) < 0.0001);
        }
    }

    #[test]
    fn cardinal_travel_targets_the_nearest_point_on_the_selected_edge() {
        let center = Vec3::ZERO;
        let extents = Vec2::new(28.0, 32.0);
        let start = SCREEN_RIGHT * 7.0 + SCREEN_UP * 8.0 + Vec3::Y * (PLAYER_HEIGHT / 2.0);
        for direction in [
            ScreenDirection::Up,
            ScreenDirection::Down,
            ScreenDirection::Left,
            ScreenDirection::Right,
        ] {
            let destination = travel_destination(start, center, extents, direction);
            let local = destination - center;
            let offset = direction.grid_offset();
            if offset.x == 0 {
                assert!((local.dot(SCREEN_RIGHT) - 7.0).abs() < 0.0001);
                assert!((local.dot(SCREEN_UP) + offset.y as f32 * 22.0).abs() < 0.0001);
            } else {
                assert!((local.dot(SCREEN_UP) - 8.0).abs() < 0.0001);
                assert!((local.dot(SCREEN_RIGHT) - offset.x as f32 * 24.0).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn diagonal_screen_travel_changes_both_coordinates_and_enters_the_matching_corner() {
        let extents = Vec2::new(28.0, 32.0);
        let center = Vec3::ZERO;
        let start = Vec3::Y * (PLAYER_HEIGHT / 2.0);
        for direction in [
            ScreenDirection::UpLeft,
            ScreenDirection::UpRight,
            ScreenDirection::DownLeft,
            ScreenDirection::DownRight,
        ] {
            let offset = direction.grid_offset();
            let (next_center, entry) = screen_transition(start, center, extents, direction);
            assert_eq!(
                next_center,
                SCREEN_RIGHT * (offset.x as f32 * extents.x * 2.0)
                    - SCREEN_UP * (offset.y as f32 * extents.y * 2.0)
            );
            let local = entry - next_center;
            assert_eq!(local.dot(SCREEN_RIGHT).signum(), -(offset.x as f32));
            assert_eq!(local.dot(SCREEN_UP).signum(), offset.y as f32);
        }
    }

    #[test]
    fn destination_bounds_use_camera_axes_on_distant_screens() {
        let center = SCREEN_RIGHT * 200.0 - SCREEN_UP * 320.0;
        let extents = Vec2::new(28.0, 32.0);
        let point = center + SCREEN_RIGHT * 100.0 - SCREEN_UP * 100.0;
        let clamped = clamp_to_screen(point, center, extents) - center;
        assert!((clamped.dot(SCREEN_RIGHT) - 24.0).abs() < 0.0001);
        assert!((clamped.dot(SCREEN_UP) + 22.0).abs() < 0.0001);
    }

    #[test]
    fn keyboard_and_overlay_travel_update_the_world_and_cancel_movement() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .insert_resource(MovementSettings {
                speed: DEFAULT_MOVE_SPEED,
            })
            .insert_resource(ScreenAnimationSettings {
                enabled: false,
                duration: 0.18,
            })
            .init_resource::<ScreenAnimation>()
            .init_resource::<WorldScreen>()
            .init_resource::<GameViewport>()
            .init_resource::<OverlayPointer>()
            .add_message::<OverlayInput>()
            .add_systems(
                Update,
                (
                    handle_overlay_and_travel,
                    set_destination,
                    move_player,
                    finish_travel,
                    update_status,
                )
                    .chain(),
            );
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        let player = app
            .world_mut()
            .spawn((
                Player {
                    destination: Some(Vec3::new(10.0, PLAYER_HEIGHT / 2.0, 10.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, PLAYER_HEIGHT / 2.0, 0.0),
            ))
            .id();
        let camera_origin = Vec3::new(40.0, 32.0, 40.0);
        let camera = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                Transform::from_translation(camera_origin),
            ))
            .id();
        let ground = app.world_mut().spawn((Ground, Transform::default())).id();
        app.world_mut().spawn((StatusText, Text::new("")));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowRight);
        app.update();
        assert_eq!(
            app.world().resource::<WorldScreen>().coordinates,
            IVec2::ZERO
        );
        let destination = app
            .world()
            .get::<Player>(player)
            .unwrap()
            .destination
            .unwrap();
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = destination;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        let screen = app.world().resource::<WorldScreen>();
        assert_eq!(screen.coordinates, IVec2::new(1, 0));
        assert!(
            app.world()
                .get::<Transform>(camera)
                .unwrap()
                .translation
                .distance(camera_origin + screen.center)
                < 0.0001
        );
        assert_eq!(
            app.world().get::<Transform>(ground).unwrap().translation,
            screen.center
        );
        assert!(
            app.world()
                .get::<Player>(player)
                .unwrap()
                .destination
                .is_none()
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        // UI travel must not also set a ground destination on the same click.
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        let overlay = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<OverlayPointer>().hovered = Some(overlay);
        app.world_mut()
            .write_message(OverlayInput(OverlayAction::Travel(ScreenDirection::Up)));
        app.update();
        assert_eq!(
            app.world().resource::<WorldScreen>().coordinates,
            IVec2::new(1, 0)
        );
        let destination = app
            .world()
            .get::<Player>(player)
            .unwrap()
            .destination
            .unwrap();
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation = destination;
        app.update();
        assert_eq!(
            app.world().resource::<WorldScreen>().coordinates,
            IVec2::new(1, -1)
        );
        assert!(
            app.world()
                .get::<Player>(player)
                .unwrap()
                .destination
                .is_none()
        );
    }

    #[test]
    fn screen_colors_are_stable_and_all_nine_neighbor_screens_differ() {
        let center = IVec2::new(-5, 7);
        assert_eq!(screen_color(center), screen_color(center));
        let colors: Vec<_> = (-1..=1)
            .flat_map(|y| (-1..=1).map(move |x| screen_color(center + IVec2::new(x, y))))
            .collect();
        for (index, color) in colors.iter().enumerate() {
            assert!(colors[index + 1..].iter().all(|other| color != other));
        }
    }

    #[test]
    fn screen_tween_eases_and_clamps_progress() {
        assert_eq!(transition_progress(0.0, 0.2), 0.0);
        assert_eq!(transition_progress(0.1, 0.2), 0.5);
        assert!(transition_progress(0.02, 0.2) < 0.1);
        assert_eq!(transition_progress(0.3, 0.2), 1.0);
    }

    #[test]
    fn animated_travel_reaches_exact_targets_and_blocks_input() {
        use bevy::time::TimeUpdateStrategy;
        use std::time::Duration;

        for duration in [0.1, 0.4] {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                    0.02,
                )))
                .insert_resource(ScreenAnimationSettings {
                    enabled: true,
                    duration,
                })
                .init_resource::<Assets<StandardMaterial>>()
                .init_resource::<ScreenAnimation>()
                .init_resource::<WorldScreen>()
                .init_resource::<GameViewport>()
                .init_resource::<ButtonInput<KeyCode>>()
                .add_message::<OverlayInput>()
                .add_systems(
                    Update,
                    (handle_overlay_and_travel, finish_travel, animate_screen).chain(),
                );
            let origin = Vec3::Y * (PLAYER_HEIGHT / 2.0);
            let player = app
                .world_mut()
                .spawn((
                    Player {
                        destination: None,
                        path: Vec::new(),
                        travel: Some(ScreenDirection::Right),
                        approach_enemy: None,
                    },
                    Transform::from_translation(origin),
                ))
                .id();
            let camera_origin = Vec3::new(40.0, 32.0, 40.0);
            let camera = app
                .world_mut()
                .spawn((
                    Camera3d::default(),
                    Transform::from_translation(camera_origin),
                ))
                .id();
            let material = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial::default());
            let ground = app
                .world_mut()
                .spawn((
                    Ground,
                    Transform::default(),
                    MeshMaterial3d(material.clone()),
                ))
                .id();
            app.update();
            assert!(app.world().resource::<ScreenAnimation>().0.is_some());
            assert_eq!(
                app.world().get::<Transform>(camera).unwrap().translation,
                camera_origin
            );
            app.world_mut()
                .write_message(OverlayInput(OverlayAction::Travel(ScreenDirection::Up)));
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::ArrowLeft);
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .clear();
            assert!(
                app.world()
                    .get::<Player>(player)
                    .unwrap()
                    .destination
                    .is_none()
            );
            let center = app.world().resource::<WorldScreen>().center;
            let halfway_camera = app.world().get::<Transform>(camera).unwrap().translation;
            assert!(halfway_camera.distance(camera_origin) > 0.0);
            assert!(halfway_camera.distance(camera_origin) < center.length());
            for _ in 0..((duration / 0.02).ceil() as usize + 2) {
                app.update();
            }
            assert!(app.world().resource::<ScreenAnimation>().0.is_none());
            assert_eq!(
                app.world().resource::<WorldScreen>().coordinates,
                IVec2::new(1, 0)
            );
            assert!(
                app.world()
                    .get::<Transform>(camera)
                    .unwrap()
                    .translation
                    .distance(camera_origin + center)
                    < 0.0001
            );
            assert_eq!(
                app.world().get::<Transform>(ground).unwrap().translation,
                center
            );
            let (_, entry) = screen_transition(
                origin,
                Vec3::ZERO,
                screen_half_extents(Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32)),
                ScreenDirection::Right,
            );
            assert_eq!(
                app.world().get::<Transform>(player).unwrap().translation,
                entry
            );
        }
    }

    #[test]
    fn movement_uses_the_requested_distance_and_preserves_height() {
        let start = Vec3::new(0.0, PLAYER_HEIGHT / 2.0, 0.0);
        let destination = Vec3::new(3.0, start.y, 4.0);
        let next = step_toward(start, destination, 2.0);
        assert!((next.distance(start) - 2.0).abs() < 0.0001);
        assert_eq!(next.y, start.y);
        assert!(next.distance(Vec3::new(1.2, start.y, 1.6)) < 0.0001);
    }

    #[test]
    fn movement_stops_without_overshooting_or_normalizing_zero() {
        let destination = Vec3::new(1.0, PLAYER_HEIGHT / 2.0, 1.0);
        assert_eq!(step_toward(Vec3::ZERO, destination, 10.0), destination);
        assert_eq!(step_toward(destination, destination, 0.0), destination);
    }

    #[test]
    fn pathfinding_routes_around_building_footprints_with_player_clearance() {
        let obstacle = Obstacle {
            center: Vec2::ZERO,
            half_size: Vec2::new(2.0, 1.0) + Vec2::splat(PLAYER_RADIUS + 0.15),
        };
        let start = Vec3::new(-5.0, PLAYER_HEIGHT / 2.0, 0.0);
        let end = Vec3::new(5.0, PLAYER_HEIGHT / 2.0, 0.0);
        let path = find_path(start, end, &[obstacle]);
        assert!(path.len() >= 2, "path should detour around the building");
        let mut previous = start;
        for waypoint in path {
            assert!(
                !segment_hits_obstacle(
                    Vec2::new(previous.x, previous.z),
                    Vec2::new(waypoint.x, waypoint.z),
                    obstacle
                ),
                "path segment intersects the expanded building footprint"
            );
            previous = waypoint;
        }
    }

    #[test]
    fn click_inside_building_is_rerouted_to_its_nearest_clear_corner() {
        let obstacle = Obstacle {
            center: Vec2::ZERO,
            half_size: Vec2::splat(2.0),
        };
        let path = find_path(
            Vec3::new(-5.0, PLAYER_HEIGHT / 2.0, 0.0),
            Vec3::new(0.0, PLAYER_HEIGHT / 2.0, 0.0),
            &[obstacle],
        );
        assert!(!path.is_empty());
        let end = *path.last().unwrap();
        assert!(!obstacle.contains(Vec2::new(end.x, end.z)));
    }
}

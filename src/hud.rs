use crate::config::{WINDOW_HEIGHT, WINDOW_WIDTH};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) const SNAP: f32 = 16.0;
const PANEL_WIDTH: f32 = 256.0;
const PANEL_HEIGHT: f32 = 160.0;
const HUD_LAYOUT_PATH: &str = "hud_layout.json";
const STATS_KEY: &str = "Stats";

#[derive(Resource, Default, Serialize, Deserialize)]
struct HudLayout {
    positions: HashMap<String, [f32; 2]>,
    #[serde(default)]
    visibility: HashMap<String, bool>,
}

impl HudLayout {
    fn position(&self, key: &str) -> Option<Vec2> {
        self.positions
            .get(key)
            .map(|position| Vec2::from_array(*position))
    }

    fn visible(&self, key: &str) -> bool {
        self.visibility.get(key).copied().unwrap_or(true)
    }

    fn set_visible(&mut self, key: &str, visible: bool) -> bool {
        if self.visibility.get(key) == Some(&visible) {
            return false;
        }
        self.visibility.insert(key.to_owned(), visible);
        true
    }

    fn set_position(&mut self, key: &str, position: Vec2) -> bool {
        let next = position.to_array();
        if self.positions.get(key).is_some_and(|current| {
            (current[0] - next[0]).abs() < f32::EPSILON
                && (current[1] - next[1]).abs() < f32::EPSILON
        }) {
            return false;
        }
        self.positions.insert(key.to_owned(), next);
        true
    }
}

#[derive(Resource, Default)]
pub(super) struct HudState {
    cursor_bounds: Vec<Rect>,
    suppress_world_click: bool,
}

impl HudState {
    pub(super) fn contains_cursor(&self, cursor: Vec2) -> bool {
        self.suppress_world_click
            || self
                .cursor_bounds
                .iter()
                .any(|bounds| bounds.contains(cursor))
    }
}

#[derive(Component)]
struct StatsPanel;

#[derive(Component)]
struct HudContextMenu;

#[derive(Component)]
struct CloseHudWindow(Entity);

#[derive(Component)]
pub(super) struct HudWindow {
    key: String,
    position: Vec2,
    size: Vec2,
    hotkey: Option<KeyCode>,
    visible: bool,
    dragging: bool,
    drag_offset: Vec2,
}

#[derive(Component)]
struct SnapGrid;

fn snap_hud_size(size: Vec2) -> Vec2 {
    (size.max(Vec2::splat(SNAP)) / SNAP).ceil() * SNAP
}

fn constrain_hud_position(position: Vec2, size: Vec2) -> Vec2 {
    let max_position = Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32) - size;
    (position / SNAP)
        .round()
        .clamp(Vec2::ZERO, (max_position / SNAP).floor())
        * SNAP
}

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<HudLayout>()
            .add_systems(Startup, (load_hud_layout, setup_hud).chain())
            .add_systems(
                Update,
                (
                    restore_hud_window_positions,
                    interact_with_hud_windows,
                    interact_with_hud_context_menu,
                    toggle_hud_windows,
                    sync_hud,
                    save_hud_layout,
                )
                    .chain(),
            );
    }
}

pub(super) fn spawn_hud_window<C: Component>(
    commands: &mut Commands,
    title: &str,
    hotkey: KeyCode,
    content_marker: C,
    position: Vec2,
    size: Vec2,
    content: &str,
    font_size: f32,
) -> Entity {
    let size = snap_hud_size(size);
    let position = constrain_hud_position(position, size);
    let entity = commands
        .spawn((
            DespawnOnExit(crate::combat::GameMode::Combat),
            HudWindow {
                key: title.to_owned(),
                hotkey: Some(hotkey),
                position,
                size,
                visible: true,
                dragging: false,
                drag_offset: Vec2::ZERO,
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x),
                top: px(position.y),
                width: px(size.x),
                height: px(size.y),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.075, 0.11, 0.96)),
            BorderColor::all(Color::srgb(0.30, 0.44, 0.58)),
            ZIndex(20),
            Visibility::Visible,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(entity).with_children(|window| {
        window.spawn((
            content_marker,
            Text::new(content.to_owned()),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::new(Justify::Left, LineBreak::WordBoundary),
            Node {
                width: percent(100),
                min_width: px(0),
                flex_grow: 1.0,
                padding: UiRect::new(px(8), px(28), px(8), px(8)),
                align_items: AlignItems::Center,
                ..default()
            },
        ));
    });
    entity
}

fn load_hud_layout(mut layout: ResMut<HudLayout>) {
    match std::fs::read_to_string(HUD_LAYOUT_PATH) {
        Ok(contents) => match serde_json::from_str::<HudLayout>(&contents) {
            Ok(saved) => *layout = saved,
            Err(error) => warn!("Could not parse {HUD_LAYOUT_PATH}: {error}"),
        },
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            warn!("Could not read {HUD_LAYOUT_PATH}: {error}");
        }
        Err(_) => {}
    }
}

fn setup_hud(mut commands: Commands, layout: Res<HudLayout>) {
    let position = constrain_hud_position(
        layout.position(STATS_KEY).unwrap_or(Vec2::splat(SNAP)),
        Vec2::new(PANEL_WIDTH, PANEL_HEIGHT),
    );
    let visible = layout.visible(STATS_KEY);
    commands
        .spawn((
            StatsPanel,
            HudWindow {
                key: STATS_KEY.to_owned(),
                position,
                size: Vec2::new(PANEL_WIDTH, PANEL_HEIGHT),
                hotkey: Some(KeyCode::KeyS),
                visible,
                dragging: false,
                drag_offset: Vec2::ZERO,
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x),
                top: px(position.y),
                width: px(PANEL_WIDTH),
                height: px(PANEL_HEIGHT),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.075, 0.11, 0.96)),
            BorderColor::all(Color::srgb(0.30, 0.44, 0.58)),
            ZIndex(20),
            Visibility::Visible,
            Pickable::IGNORE,
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    padding: UiRect::all(px(12)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(9),
                    ..default()
                })
                .with_children(|body| {
                    stat_row(body, "Total HP", "100", Color::srgb(0.36, 0.84, 0.48));
                    stat_row(
                        body,
                        "Physical Damage",
                        "12%",
                        Color::srgb(0.95, 0.68, 0.30),
                    );
                    stat_row(body, "Magical Damage", "8%", Color::srgb(0.43, 0.68, 0.98));
                    stat_row(body, "Physical Defense", "0", Color::srgb(0.72, 0.76, 0.82));
                    stat_row(body, "Magical Defense", "0", Color::srgb(0.72, 0.76, 0.82));
                });
        });

    commands
        .spawn((
            SnapGrid,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                display: Display::None,
                ..default()
            },
            ZIndex(10),
            Pickable::IGNORE,
        ))
        .with_children(|grid| {
            for x in (0..=WINDOW_WIDTH).step_by(SNAP as usize) {
                grid.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x as f32),
                        width: px(1),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.6, 0.75, 0.9, 0.12)),
                    Pickable::IGNORE,
                ));
            }
            for y in (0..=WINDOW_HEIGHT).step_by(SNAP as usize) {
                grid.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(y as f32),
                        height: px(1),
                        width: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.6, 0.75, 0.9, 0.12)),
                    Pickable::IGNORE,
                ));
            }
        });
}

fn stat_row(parent: &mut ChildSpawnerCommands, label: &str, value: &str, color: Color) {
    parent
        .spawn((Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(color),
            ));
            row.spawn((
                Text::new(value),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn interact_with_hud_windows(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    mut windows: Query<(&mut HudWindow, &mut Node, &Visibility)>,
) {
    let cursor = window.and_then(|window| window.cursor_position());
    for (mut hud_window, mut node, visibility) in &mut windows {
        if mouse.just_released(MouseButton::Left) || cursor.is_none() {
            hud_window.dragging = false;
        }
        if *visibility != Visibility::Visible {
            hud_window.dragging = false;
            continue;
        }
        let Some(cursor) = cursor else { continue };
        if mouse.just_pressed(MouseButton::Left) {
            let local = cursor - hud_window.position;
            if local.x >= 0.0
                && local.x <= hud_window.size.x
                && local.y >= 0.0
                && local.y <= hud_window.size.y
            {
                hud_window.dragging = true;
                hud_window.drag_offset = local;
            }
        }
        if hud_window.dragging && mouse.pressed(MouseButton::Left) {
            hud_window.position =
                constrain_hud_position(cursor - hud_window.drag_offset, hud_window.size);
            node.left = px(hud_window.position.x);
            node.top = px(hud_window.position.y);
        }
    }
}

fn interact_with_hud_context_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    mut state: ResMut<HudState>,
    mut windows: Query<(Entity, &mut HudWindow, &Visibility)>,
    menus: Query<Entity, With<HudContextMenu>>,
    close_buttons: Query<(&Interaction, &CloseHudWindow), Changed<Interaction>>,
) {
    state.suppress_world_click =
        menus.iter().next().is_some() && mouse.just_pressed(MouseButton::Left);
    let closed = close_buttons
        .iter()
        .filter_map(|(interaction, close)| {
            (*interaction == Interaction::Pressed).then_some(close.0)
        })
        .collect::<Vec<_>>();
    for entity in closed {
        if let Ok((_, mut hud_window, _)) = windows.get_mut(entity) {
            hud_window.visible = false;
            hud_window.dragging = false;
            state.suppress_world_click = true;
        }
    }

    let cursor = window.and_then(|window| window.cursor_position());
    let mut opened_menu = false;
    if mouse.just_pressed(MouseButton::Right)
        && let Some(cursor) = cursor
    {
        let target = windows.iter().find_map(|(entity, hud_window, visibility)| {
            let inside = cursor.x >= hud_window.position.x
                && cursor.x <= hud_window.position.x + hud_window.size.x
                && cursor.y >= hud_window.position.y
                && cursor.y <= hud_window.position.y + hud_window.size.y;
            (inside && hud_window.visible && *visibility == Visibility::Visible).then_some(entity)
        });
        if let Some(target) = target {
            for menu in &menus {
                commands.entity(menu).despawn();
            }
            commands
                .spawn((
                    HudContextMenu,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(cursor.x.min(WINDOW_WIDTH as f32 - 96.0)),
                        top: px(cursor.y.min(WINDOW_HEIGHT as f32 - 32.0)),
                        width: px(96),
                        height: px(32),
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(px(10)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.15, 0.20)),
                    BorderColor::all(Color::srgb(0.42, 0.52, 0.64)),
                    ZIndex(100),
                ))
                .with_children(|menu| {
                    menu.spawn((
                        CloseHudWindow(target),
                        Button,
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
                            Text::new("Close"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
            opened_menu = true;
        }
    }
    if !opened_menu
        && (mouse.just_pressed(MouseButton::Left)
            || mouse.just_pressed(MouseButton::Right)
            || keyboard.just_pressed(KeyCode::Escape))
    {
        for menu in &menus {
            commands.entity(menu).despawn();
        }
    }
}

fn toggle_hud_windows(
    keyboard: Res<ButtonInput<KeyCode>>,
    mode: Res<State<crate::combat::GameMode>>,
    mut windows: Query<(&mut HudWindow, &mut Visibility, Option<&StatsPanel>)>,
) {
    for (mut window, mut visibility, stats_panel) in &mut windows {
        let allowed = stats_panel.is_none() || *mode.get() == crate::combat::GameMode::World;
        if allowed
            && window
                .hotkey
                .is_some_and(|hotkey| keyboard.just_pressed(hotkey))
        {
            window.visible = !window.visible;
            window.dragging = false;
            visibility.set_if_neq(if window.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
        }
    }
}

fn restore_hud_window_positions(
    layout: Res<HudLayout>,
    mut windows: Query<(&mut HudWindow, &mut Node, &mut Visibility), Added<HudWindow>>,
) {
    for (mut window, mut node, mut visibility) in &mut windows {
        window.visible = layout.visible(&window.key);
        visibility.set_if_neq(if window.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        if let Some(position) = layout.position(&window.key) {
            window.position = constrain_hud_position(position, window.size);
            node.left = px(window.position.x);
            node.top = px(window.position.y);
        }
    }
}

fn save_hud_layout(windows: Query<&HudWindow>, mut layout: ResMut<HudLayout>) {
    if windows.iter().any(|window| window.dragging) {
        return;
    }
    let mut changed = false;
    for window in &windows {
        changed |= layout.set_position(&window.key, window.position);
        changed |= layout.set_visible(&window.key, window.visible);
    }
    if changed {
        match serde_json::to_string_pretty(&*layout)
            .map_err(std::io::Error::other)
            .and_then(|contents| std::fs::write(HUD_LAYOUT_PATH, contents))
        {
            Ok(()) => {}
            Err(error) => warn!("Could not save {HUD_LAYOUT_PATH}: {error}"),
        }
    }
}

fn sync_hud(
    mut state: ResMut<HudState>,
    mut grid: Single<&mut Node, With<SnapGrid>>,
    mode: Res<State<crate::combat::GameMode>>,
    mut windows: Query<
        (
            &mut HudWindow,
            &mut Node,
            &mut Visibility,
            Option<&StatsPanel>,
        ),
        Without<SnapGrid>,
    >,
) {
    state.cursor_bounds.clear();
    let mut dragging = false;
    for (window, mut node, mut visibility, stats_panel) in &mut windows {
        let active = window.visible
            && (stats_panel.is_none() || *mode.get() == crate::combat::GameMode::World);
        visibility.set_if_neq(if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        node.left = px(window.position.x);
        node.top = px(window.position.y);
        if active {
            state.cursor_bounds.push(Rect::from_corners(
                window.position,
                window.position + window.size,
            ));
        }
        dragging |= window.dragging;
    }
    grid.display = if dragging {
        Display::Grid
    } else {
        Display::None
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_modules_keep_every_edge_on_the_grid_and_inside_the_viewport() {
        let size = snap_hud_size(Vec2::new(320.0, 44.0));
        let position = constrain_hud_position(Vec2::new(f32::MAX, f32::MAX), size);

        assert_eq!(size, Vec2::new(320.0, 48.0));
        assert_eq!(position % SNAP, Vec2::ZERO);
        assert_eq!(
            position + size,
            Vec2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32)
        );
    }

    #[test]
    fn hud_layout_round_trips_named_positions() {
        let mut layout = HudLayout::default();
        assert!(layout.set_position(STATS_KEY, Vec2::new(48.0, 64.0)));
        assert!(!layout.set_position(STATS_KEY, Vec2::new(48.0, 64.0)));

        let encoded = serde_json::to_string(&layout).unwrap();
        let decoded: HudLayout = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.position(STATS_KEY), Some(Vec2::new(48.0, 64.0)));
        assert!(decoded.visible(STATS_KEY));
    }

    #[test]
    fn closing_any_hud_panel_consumes_the_close_click_for_world_movement() {
        let mut state = HudState::default();
        state.cursor_bounds.push(Rect::from_corners(
            Vec2::new(32.0, 48.0),
            Vec2::new(32.0 + PANEL_WIDTH, 48.0 + PANEL_HEIGHT),
        ));

        assert!(state.contains_cursor(Vec2::new(40.0, 56.0)));
        assert!(!state.contains_cursor(Vec2::new(16.0, 16.0)));
        state.cursor_bounds.clear();
        state.suppress_world_click = true;
        assert!(state.contains_cursor(Vec2::new(16.0, 16.0)));
    }

    #[test]
    fn hud_layout_round_trips_closed_visibility_and_loads_legacy_layouts() {
        let mut layout = HudLayout::default();
        assert!(layout.set_visible(STATS_KEY, false));
        assert!(!layout.set_visible(STATS_KEY, false));

        let encoded = serde_json::to_string(&layout).unwrap();
        let decoded: HudLayout = serde_json::from_str(&encoded).unwrap();
        assert!(!decoded.visible(STATS_KEY));

        let legacy: HudLayout = serde_json::from_str(r#"{"positions":{}}"#).unwrap();
        assert!(legacy.visible(STATS_KEY));
    }
}

use crate::config::{WINDOW_HEIGHT, WINDOW_WIDTH};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) const SNAP: f32 = 16.0;
const PANEL_WIDTH: f32 = 256.0;
const PANEL_HEIGHT: f32 = 160.0;
const HEADER_HEIGHT: f32 = 32.0;

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

#[derive(Resource)]
pub(super) struct HudState {
    pub(super) position: Vec2,
    pub(super) visible: bool,
    dragging: bool,
    suppress_world_click: bool,
    drag_offset: Vec2,
}

impl HudState {
    pub(super) fn contains_cursor(&self, cursor: Vec2) -> bool {
        (self.visible || self.suppress_world_click)
            && cursor.x >= self.position.x
            && cursor.x <= self.position.x + PANEL_WIDTH
            && cursor.y >= self.position.y
            && cursor.y <= self.position.y + PANEL_HEIGHT
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            position: Vec2::splat(SNAP),
            visible: true,
            dragging: false,
            suppress_world_click: false,
            drag_offset: Vec2::ZERO,
        }
    }
}

#[derive(Component)]
struct StatsPanel;

#[derive(Component)]
struct CloseStats;

#[derive(Component)]
struct CloseHudWindow(Entity);

#[derive(Component)]
pub(super) struct HudWindow {
    key: String,
    position: Vec2,
    size: Vec2,
    hotkey: KeyCode,
    dragging: bool,
    drag_offset: Vec2,
}

#[derive(Component)]
struct SnapGrid;

type StatsPanelFilter = (With<StatsPanel>, Without<SnapGrid>);

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<HudLayout>()
            .add_systems(Startup, (load_hud_layout, setup_hud).chain())
            .add_systems(
                Update,
                (
                    toggle_stats,
                    restore_hud_window_positions,
                    interact_with_hud,
                    interact_with_hud_windows,
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
) {
    let entity = commands
        .spawn((
            DespawnOnExit(crate::combat::GameMode::Combat),
            HudWindow {
                key: title.to_owned(),
                hotkey,
                position,
                size,
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
        window
            .spawn((
                Node {
                    position_type: PositionType::Relative,
                    height: px(HEADER_HEIGHT),
                    min_height: px(HEADER_HEIGHT),
                    padding: UiRect::axes(px(8), px(4)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.15, 0.22)),
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new(title.to_owned()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.75, 0.83, 0.92)),
                ));
                header
                    .spawn((
                        CloseHudWindow(entity),
                        Button,
                        Node {
                            position_type: PositionType::Absolute,
                            right: px(2),
                            top: px(2),
                            width: px(16),
                            height: px(16),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.35, 0.16, 0.18)),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("x"),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            });
        window.spawn((
            content_marker,
            Text::new(content.to_owned()),
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                flex_grow: 1.0,
                padding: UiRect::all(px(8)),
                align_items: AlignItems::Center,
                ..default()
            },
        ));
    });
}

fn load_hud_layout(mut state: ResMut<HudState>, mut layout: ResMut<HudLayout>) {
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
    if let Some(position) = layout.position(STATS_KEY) {
        state.position = position;
    }
    state.visible = layout.visible(STATS_KEY);
}

fn setup_hud(mut commands: Commands, state: Res<HudState>) {
    commands
        .spawn((
            StatsPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(state.position.x),
                top: px(state.position.y),
                width: px(PANEL_WIDTH),
                height: px(PANEL_HEIGHT),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.075, 0.11, 0.96)),
            BorderColor::all(Color::srgb(0.30, 0.44, 0.58)),
            ZIndex(20),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        height: px(HEADER_HEIGHT),
                        padding: UiRect::axes(px(8), px(4)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.10, 0.15, 0.22)),
                ))
                .with_children(|header| {
                    header.spawn((
                        Text::new("Stats"),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.83, 0.92)),
                    ));
                    header
                        .spawn((
                            CloseStats,
                            Button,
                            Node {
                                position_type: PositionType::Absolute,
                                right: px(2),
                                top: px(2),
                                width: px(16),
                                height: px(16),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.35, 0.16, 0.18)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("x"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });
                });
            panel
                .spawn(Node {
                    padding: UiRect::all(px(12)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(9),
                    ..default()
                })
                .with_children(|body| {
                    stat_row(body, "HP", "100 / 100", Color::srgb(0.36, 0.84, 0.48));
                    stat_row(body, "PD", "12", Color::srgb(0.95, 0.68, 0.30));
                    stat_row(body, "MD", "8", Color::srgb(0.43, 0.68, 0.98));
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

fn toggle_stats(
    keyboard: Res<ButtonInput<KeyCode>>,
    mode: Res<State<crate::combat::GameMode>>,
    mut state: ResMut<HudState>,
) {
    if *mode.get() == crate::combat::GameMode::World && keyboard.just_pressed(KeyCode::KeyS) {
        state.visible = !state.visible;
        state.dragging = false;
    }
}

fn interact_with_hud(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    mut state: ResMut<HudState>,
    close_buttons: Query<&Interaction, (Changed<Interaction>, With<CloseStats>)>,
) {
    state.suppress_world_click = false;
    if close_buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.visible = false;
        state.suppress_world_click = true;
        state.dragging = false;
    }
    let Some(cursor) = window.and_then(|window| window.cursor_position()) else {
        if mouse.just_released(MouseButton::Left) {
            state.dragging = false;
        }
        return;
    };

    if mouse.just_pressed(MouseButton::Left) && state.visible {
        let local = cursor - state.position;
        if local.x >= 0.0 && local.x <= PANEL_WIDTH && local.y >= 0.0 && local.y <= HEADER_HEIGHT {
            state.dragging = true;
            state.drag_offset = local;
        }
    }
    if mouse.just_released(MouseButton::Left) {
        state.dragging = false;
    }
    if state.dragging && mouse.pressed(MouseButton::Left) {
        state.position = ((cursor - state.drag_offset) / SNAP).round() * SNAP;
    }
}

fn interact_with_hud_windows(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    close_buttons: Query<(&Interaction, &CloseHudWindow), Changed<Interaction>>,
    mut windows: Query<(Entity, &mut HudWindow, &mut Node, &mut Visibility)>,
) {
    let closed = close_buttons
        .iter()
        .filter_map(|(interaction, close)| {
            (*interaction == Interaction::Pressed).then_some(close.0)
        })
        .collect::<Vec<_>>();
    let cursor = window.and_then(|window| window.cursor_position());
    for (entity, mut hud_window, mut node, mut visibility) in &mut windows {
        if closed.contains(&entity) {
            hud_window.dragging = false;
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }
        if mouse.just_released(MouseButton::Left) || cursor.is_none() {
            hud_window.dragging = false;
        }
        let Some(cursor) = cursor else { continue };
        if mouse.just_pressed(MouseButton::Left) {
            let local = cursor - hud_window.position;
            if local.x >= 0.0
                && local.x <= hud_window.size.x
                && local.y >= 0.0
                && local.y <= HEADER_HEIGHT
            {
                hud_window.dragging = true;
                hud_window.drag_offset = local;
            }
        }
        if hud_window.dragging && mouse.pressed(MouseButton::Left) {
            hud_window.position = ((cursor - hud_window.drag_offset) / SNAP).round() * SNAP;
            node.left = px(hud_window.position.x.max(0.0));
            node.top = px(hud_window.position.y.max(0.0));
        }
    }
}

fn toggle_hud_windows(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut windows: Query<(&HudWindow, &mut Visibility)>,
) {
    for (window, mut visibility) in &mut windows {
        if keyboard.just_pressed(window.hotkey) {
            visibility.set_if_neq(if *visibility == Visibility::Visible {
                Visibility::Hidden
            } else {
                Visibility::Visible
            });
        }
    }
}

fn restore_hud_window_positions(
    layout: Res<HudLayout>,
    mut windows: Query<(&mut HudWindow, &mut Node, &mut Visibility), Added<HudWindow>>,
) {
    for (mut window, mut node, mut visibility) in &mut windows {
        visibility.set_if_neq(if layout.visible(&window.key) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        if let Some(position) = layout.position(&window.key) {
            window.position = position;
            node.left = px(position.x.max(0.0));
            node.top = px(position.y.max(0.0));
        }
    }
}

fn save_hud_layout(
    state: Res<HudState>,
    windows: Query<(&HudWindow, &Visibility)>,
    mut layout: ResMut<HudLayout>,
) {
    if state.dragging || windows.iter().any(|(window, _)| window.dragging) {
        return;
    }
    let mut changed = layout.set_position(STATS_KEY, state.position);
    changed |= layout.set_visible(STATS_KEY, state.visible);
    for (window, visibility) in &windows {
        changed |= layout.set_position(&window.key, window.position);
        changed |= layout.set_visible(&window.key, *visibility == Visibility::Visible);
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
    state: Res<HudState>,
    mut panel: Single<(&mut Node, &mut Visibility), StatsPanelFilter>,
    mut grid: Single<&mut Node, (With<SnapGrid>, Without<StatsPanel>)>,
    mode: Res<State<crate::combat::GameMode>>,
    windows: Query<&HudWindow>,
) {
    let (node, visibility) = &mut *panel;
    visibility.set_if_neq(
        if state.visible && *mode.get() == crate::combat::GameMode::World {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    );
    node.left = px(state.position.x.max(0.0));
    node.top = px(state.position.y.max(0.0));
    grid.display = if state.dragging || windows.iter().any(|window| window.dragging) {
        Display::Grid
    } else {
        Display::None
    };
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn closing_stats_still_consumes_the_close_click_for_world_movement() {
        let mut state = HudState::default();
        state.visible = false;
        state.suppress_world_click = true;

        assert!(state.contains_cursor(state.position + Vec2::new(8.0, 8.0)));
        assert!(!state.contains_cursor(state.position + Vec2::new(PANEL_WIDTH + 1.0, 8.0)));
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

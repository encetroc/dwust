use crate::config::{WINDOW_HEIGHT, WINDOW_WIDTH};
use bevy::prelude::*;

pub(super) const SNAP: f32 = 16.0;
const PANEL_WIDTH: f32 = 256.0;
const PANEL_HEIGHT: f32 = 160.0;
const HEADER_HEIGHT: f32 = 32.0;

#[derive(Resource)]
pub(super) struct HudState {
    pub(super) position: Vec2,
    pub(super) visible: bool,
    dragging: bool,
    drag_offset: Vec2,
}

impl HudState {
    pub(super) fn contains_cursor(&self, cursor: Vec2) -> bool {
        self.visible
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
    position: Vec2,
    size: Vec2,
    dragging: bool,
    drag_offset: Vec2,
}

#[derive(Component)]
struct SnapGrid;

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    toggle_stats,
                    interact_with_hud,
                    interact_with_hud_windows,
                    sync_hud,
                )
                    .chain(),
            );
    }
}

pub(super) fn spawn_hud_window<C: Component>(
    commands: &mut Commands,
    title: &str,
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

fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            StatsPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(SNAP),
                top: px(SNAP),
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

fn toggle_stats(keyboard: Res<ButtonInput<KeyCode>>, mut state: ResMut<HudState>) {
    if keyboard.just_pressed(KeyCode::KeyH) {
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
    if close_buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        state.visible = false;
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

fn sync_hud(
    state: Res<HudState>,
    mut panel: Single<(&mut Node, &mut Visibility), (With<StatsPanel>, Without<SnapGrid>)>,
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

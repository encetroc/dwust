use crate::{
    config::{WINDOW_HEIGHT, WINDOW_WIDTH},
    hud_layout::{self, HudGrid, SNAP},
};
use bevy::{prelude::*, ui::UiSystems};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
struct HudWindowContent(Entity);

#[derive(Component)]
struct HudGridContent;

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
    measured: bool,
    dragging: bool,
    drag_offset: Vec2,
}

#[derive(Component)]
struct SnapGrid;

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
            )
            .add_systems(
                PostUpdate,
                measure_hud_windows.in_set(UiSystems::PostLayout),
            );
    }
}

pub(super) fn spawn_hud_window<C: Component>(
    commands: &mut Commands,
    title: &str,
    hotkey: Option<KeyCode>,
    content_marker: C,
    position: Vec2,
    grid: HudGrid,
    despawn_on_combat: bool,
) -> (Entity, Entity) {
    let position = hud_layout::constrain_panel_position(position, Vec2::splat(SNAP));
    let content = commands.spawn((HudGridContent, grid.node())).id();
    let entity = commands
        .spawn((
            content_marker,
            HudWindow {
                key: title.to_owned(),
                hotkey,
                position,
                size: Vec2::splat(SNAP),
                visible: true,
                measured: false,
                dragging: false,
                drag_offset: Vec2::ZERO,
            },
            HudWindowContent(content),
            Node {
                position_type: PositionType::Absolute,
                left: px(position.x),
                top: px(position.y),
                width: auto(),
                height: auto(),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
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
    commands.entity(entity).add_child(content);
    if despawn_on_combat {
        commands
            .entity(entity)
            .insert(DespawnOnExit(crate::combat::GameMode::Combat));
    }
    (entity, content)
}

fn measure_hud_windows(
    mut windows: Query<(&mut HudWindow, &HudWindowContent, &mut Node)>,
    content_nodes: Query<&ComputedNode>,
) {
    for (mut window, HudWindowContent(content), mut node) in &mut windows {
        let Ok(computed) = content_nodes.get(*content) else {
            continue;
        };
        let content_size = computed.size() * computed.inverse_scale_factor;
        let size = hud_layout::snap_panel_size(content_size);
        if size != window.size {
            window.size = size;
            window.position = hud_layout::constrain_panel_position(window.position, size);
            node.left = px(window.position.x);
            node.top = px(window.position.y);
            node.width = px(size.x);
            node.height = px(size.y);
        }
        window.measured = true;
    }
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
    let grid = HudGrid::content_sized(2, 5, 20.0, 9.0, UiRect::all(px(12.0)));
    let (_, content) = spawn_hud_window(
        &mut commands,
        STATS_KEY,
        Some(KeyCode::KeyS),
        StatsPanel,
        layout.position(STATS_KEY).unwrap_or(Vec2::splat(SNAP)),
        grid,
        false,
    );
    commands.entity(content).with_children(|body| {
        stat_cell(body, "Total HP", Color::srgb(0.36, 0.84, 0.48), 14.0);
        stat_cell(body, "100", Color::WHITE, 16.0);
        stat_cell(body, "Physical Damage", Color::srgb(0.95, 0.68, 0.30), 14.0);
        stat_cell(body, "12%", Color::WHITE, 16.0);
        stat_cell(body, "Magical Damage", Color::srgb(0.43, 0.68, 0.98), 14.0);
        stat_cell(body, "8%", Color::WHITE, 16.0);
        stat_cell(
            body,
            "Physical Defense",
            Color::srgb(0.72, 0.76, 0.82),
            14.0,
        );
        stat_cell(body, "0", Color::WHITE, 16.0);
        stat_cell(body, "Magical Defense", Color::srgb(0.72, 0.76, 0.82), 14.0);
        stat_cell(body, "0", Color::WHITE, 16.0);
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

fn stat_cell(parent: &mut ChildSpawnerCommands, text: &str, color: Color, font_size: f32) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextColor(color),
        TextLayout::new(Justify::Left, LineBreak::WordBoundary),
    ));
}

fn interact_with_hud_windows(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    buttons: Query<&Interaction, With<Button>>,
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
        if mouse.just_pressed(MouseButton::Left)
            && !buttons
                .iter()
                .any(|interaction| *interaction == Interaction::Pressed)
        {
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
            hud_window.position = hud_layout::constrain_panel_position(
                cursor - hud_window.drag_offset,
                hud_window.size,
            );
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
            window.position = hud_layout::constrain_panel_position(position, window.size);
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
            && window.measured
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
            Vec2::new(160.0, 128.0),
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

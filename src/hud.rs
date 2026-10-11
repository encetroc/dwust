use crate::{
    config::{WINDOW_HEIGHT, WINDOW_WIDTH},
    hud_layout::{self, HudGrid, SNAP},
};
use bevy::{
    prelude::*,
    ui::{Pressed, UiSystems},
    ui_widgets::Button as UiButton,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const HUD_LAYOUT_PATH: &str = "hud_layout.json";
const STATS_KEY: &str = "Stats";

#[derive(Clone, Copy)]
pub(super) enum StatCategory {
    Hp,
    Mp,
    Ap,
    Physical,
    Magical,
    Neutral,
}

pub(super) fn stat_color(category: StatCategory) -> Color {
    match category {
        StatCategory::Hp => Color::srgb(0.92, 0.24, 0.28),
        StatCategory::Mp => Color::srgb(0.30, 0.82, 0.42),
        StatCategory::Ap => Color::srgb(0.30, 0.58, 0.96),
        StatCategory::Physical => Color::srgb(0.95, 0.58, 0.20),
        StatCategory::Magical => Color::srgb(0.70, 0.40, 0.92),
        StatCategory::Neutral => Color::srgb(0.78, 0.84, 0.92),
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum HudLayoutType {
    Combat,
    #[default]
    World,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HudModuleType {
    Both,
    CombatOnly,
}

#[derive(Default, Serialize, Deserialize)]
struct HudLayoutData {
    #[serde(default)]
    positions: HashMap<String, [f32; 2]>,
    #[serde(default)]
    visibility: HashMap<String, bool>,
}

#[derive(Resource, Default, Serialize, Deserialize)]
pub(super) struct HudLayout {
    #[serde(default)]
    combat: HudLayoutData,
    #[serde(default)]
    world: HudLayoutData,
    // Read old single-layout files, then migrate them on load.
    #[serde(default, skip_serializing)]
    positions: HashMap<String, [f32; 2]>,
    #[serde(default, skip_serializing)]
    visibility: HashMap<String, bool>,
}

impl HudLayout {
    fn data(&self, layout: HudLayoutType) -> &HudLayoutData {
        match layout {
            HudLayoutType::Combat => &self.combat,
            HudLayoutType::World => &self.world,
        }
    }

    fn data_mut(&mut self, layout: HudLayoutType) -> &mut HudLayoutData {
        match layout {
            HudLayoutType::Combat => &mut self.combat,
            HudLayoutType::World => &mut self.world,
        }
    }

    fn migrate_legacy(&mut self) {
        if !self.positions.is_empty() || !self.visibility.is_empty() {
            for data in [&mut self.combat, &mut self.world] {
                data.positions.clone_from(&self.positions);
                data.visibility.clone_from(&self.visibility);
            }
            self.positions.clear();
            self.visibility.clear();
        }
    }

    pub(super) fn position(&self, layout: HudLayoutType, key: &str) -> Option<Vec2> {
        self.data(layout)
            .positions
            .get(key)
            .map(|position| Vec2::from_array(*position))
    }

    fn visible(&self, layout: HudLayoutType, key: &str, default: bool) -> bool {
        self.data(layout)
            .visibility
            .get(key)
            .copied()
            .unwrap_or(default)
    }

    fn set_visible(&mut self, layout: HudLayoutType, key: &str, visible: bool) -> bool {
        let data = self.data_mut(layout);
        if data.visibility.get(key) == Some(&visible) {
            return false;
        }
        data.visibility.insert(key.to_owned(), visible);
        true
    }

    fn set_position(&mut self, layout: HudLayoutType, key: &str, position: Vec2) -> bool {
        let data = self.data_mut(layout);
        let next = position.to_array();
        if data.positions.get(key).is_some_and(|current| {
            (current[0] - next[0]).abs() < f32::EPSILON
                && (current[1] - next[1]).abs() < f32::EPSILON
        }) {
            return false;
        }
        data.positions.insert(key.to_owned(), next);
        true
    }
}

#[derive(Resource, Default)]
struct HudStack {
    next_position: u64,
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

#[derive(Clone, Copy)]
enum EquipmentStatValue {
    Hp,
    Mp,
    Ap,
    PhysicalDamage,
    MagicalDamage,
}

#[derive(Component)]
struct HudWindowContent(Entity);

#[derive(Component)]
struct HudGridContent;

#[derive(Component)]
pub(super) struct HudContextMenu;

#[derive(Component)]
pub(super) struct TemporaryHudWindow;

#[derive(Component)]
struct CloseHudWindow(Entity);

#[derive(Component)]
pub(super) struct HudWindow {
    key: String,
    layout_type: HudLayoutType,
    module_type: HudModuleType,
    position: Vec2,
    size: Vec2,
    stack_position: u64,
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
            .init_resource::<HudStack>()
            .init_resource::<HudLayout>()
            .add_systems(Startup, (load_hud_layout, setup_hud).chain())
            .add_systems(
                Update,
                (
                    restore_hud_window_positions,
                    initialize_hud_stack_positions,
                    interact_with_hud_windows,
                    interact_with_hud_context_menu,
                    toggle_hud_windows,
                    sync_hud,
                    update_equipment_stat_values,
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
    module_type: HudModuleType,
    initially_visible: bool,
) -> (Entity, Entity) {
    let position = hud_layout::constrain_panel_position(position, Vec2::splat(SNAP));
    let content = commands.spawn((HudGridContent, grid.node())).id();
    let entity = commands
        .spawn((
            content_marker,
            HudWindow {
                key: title.to_owned(),
                layout_type: if module_type == HudModuleType::CombatOnly {
                    HudLayoutType::Combat
                } else {
                    HudLayoutType::World
                },
                module_type,
                hotkey,
                position,
                size: Vec2::splat(SNAP),
                stack_position: 0,
                visible: initially_visible,
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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
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
    if module_type == HudModuleType::CombatOnly {
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
            Ok(mut saved) => {
                saved.migrate_legacy();
                *layout = saved;
            }
            Err(error) => warn!("Could not parse {HUD_LAYOUT_PATH}: {error}"),
        },
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            warn!("Could not read {HUD_LAYOUT_PATH}: {error}");
        }
        Err(_) => {}
    }
}

fn setup_hud(mut commands: Commands, layout: Res<HudLayout>) {
    let grid = HudGrid::content_sized(2, 7, 20.0, 9.0, UiRect::all(px(12.0)));
    let (_, content) = spawn_hud_window(
        &mut commands,
        STATS_KEY,
        Some(KeyCode::KeyS),
        StatsPanel,
        layout
            .position(HudLayoutType::World, STATS_KEY)
            .unwrap_or(Vec2::splat(SNAP)),
        grid,
        HudModuleType::Both,
        true,
    );
    commands.entity(content).with_children(|body| {
        stat_cell(body, "Total HP", stat_color(StatCategory::Hp), 14.0);
        equipment_stat_cell(body, "100", EquipmentStatValue::Hp);
        stat_cell(body, "Total MP", stat_color(StatCategory::Mp), 14.0);
        equipment_stat_cell(body, "3", EquipmentStatValue::Mp);
        stat_cell(body, "Total AP", stat_color(StatCategory::Ap), 14.0);
        equipment_stat_cell(body, "6", EquipmentStatValue::Ap);
        stat_cell(
            body,
            "Physical Damage",
            stat_color(StatCategory::Physical),
            14.0,
        );
        equipment_stat_cell(body, "12%", EquipmentStatValue::PhysicalDamage);
        stat_cell(
            body,
            "Magical Damage",
            stat_color(StatCategory::Magical),
            14.0,
        );
        equipment_stat_cell(body, "8%", EquipmentStatValue::MagicalDamage);
        stat_cell(
            body,
            "Physical Defense",
            stat_color(StatCategory::Physical),
            14.0,
        );
        stat_cell(body, "0", stat_color(StatCategory::Physical), 16.0);
        stat_cell(
            body,
            "Magical Defense",
            stat_color(StatCategory::Magical),
            14.0,
        );
        stat_cell(body, "0", stat_color(StatCategory::Magical), 16.0);
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

fn equipment_stat_cell(parent: &mut ChildSpawnerCommands, text: &str, stat: EquipmentStatValue) {
    parent.spawn((
        EquipmentStatValueEntity(stat),
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(16.0),
            ..default()
        },
        TextColor(stat_color(match stat {
            EquipmentStatValue::Hp => StatCategory::Hp,
            EquipmentStatValue::Mp => StatCategory::Mp,
            EquipmentStatValue::Ap => StatCategory::Ap,
            EquipmentStatValue::PhysicalDamage => StatCategory::Physical,
            EquipmentStatValue::MagicalDamage => StatCategory::Magical,
        })),
    ));
}

#[derive(Component)]
struct EquipmentStatValueEntity(EquipmentStatValue);

fn update_equipment_stat_values(
    equipment: Res<crate::inventory::CurrentEquipmentModifiers>,
    mut values: Query<(&mut Text, &EquipmentStatValueEntity)>,
) {
    if !equipment.is_changed() {
        return;
    }
    for (mut text, EquipmentStatValueEntity(stat)) in &mut values {
        text.0 = match stat {
            EquipmentStatValue::Hp => format!("{}", 100 + equipment.0.max_hp),
            EquipmentStatValue::Mp => format!("{}", 3 + equipment.0.mp),
            EquipmentStatValue::Ap => format!("{}", 6 + equipment.0.ap),
            EquipmentStatValue::PhysicalDamage => {
                format!("{}%", 12 + equipment.0.physical_damage)
            }
            EquipmentStatValue::MagicalDamage => {
                format!("{}%", 8 + equipment.0.magical_damage)
            }
        };
    }
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

fn initialize_hud_stack_positions(
    mut stack: ResMut<HudStack>,
    mut windows: Query<(Entity, &mut HudWindow, &mut ZIndex), Added<HudWindow>>,
) {
    let mut entities = windows
        .iter()
        .map(|(entity, _, _)| entity)
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity| entity.to_bits());
    for entity in entities {
        let Ok((_, mut window, mut z_index)) = windows.get_mut(entity) else {
            continue;
        };
        window.stack_position = stack.next_position;
        z_index.0 = 20 + stack.next_position.min(i32::MAX as u64 - 20) as i32;
        stack.next_position = stack.next_position.saturating_add(1);
    }
}

fn interact_with_hud_windows(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    buttons: Query<&Pressed, With<UiButton>>,
    mut stack: ResMut<HudStack>,
    mut windows: Query<(Entity, &mut HudWindow, &mut Node, &mut ZIndex, &Visibility)>,
) {
    let cursor = window.and_then(|window| window.cursor_position());
    let button_pressed = !buttons.is_empty();
    let topmost_hit = cursor.and_then(|cursor| {
        windows
            .iter()
            .filter(|(_, hud_window, _, _, visibility)| {
                **visibility == Visibility::Visible
                    && cursor.x >= hud_window.position.x
                    && cursor.x <= hud_window.position.x + hud_window.size.x
                    && cursor.y >= hud_window.position.y
                    && cursor.y <= hud_window.position.y + hud_window.size.y
            })
            .max_by_key(|(entity, hud_window, _, _, _)| {
                (hud_window.stack_position, entity.to_bits())
            })
            .map(|(entity, _, _, _, _)| entity)
    });
    for (entity, mut hud_window, mut node, mut z_index, visibility) in &mut windows {
        if mouse.just_released(MouseButton::Left) || cursor.is_none() {
            hud_window.dragging = false;
        }
        if *visibility != Visibility::Visible {
            hud_window.dragging = false;
            continue;
        }
        let Some(cursor) = cursor else { continue };
        if mouse.just_pressed(MouseButton::Left) {
            hud_window.dragging = false;
            if Some(entity) == topmost_hit {
                hud_window.stack_position = stack.next_position;
                z_index.0 = 20 + stack.next_position.min(i32::MAX as u64 - 20) as i32;
                stack.next_position = stack.next_position.saturating_add(1);
                if !button_pressed {
                    hud_window.dragging = true;
                    hud_window.drag_offset = cursor - hud_window.position;
                }
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

pub(super) fn ui_node_contains_cursor(
    cursor: Vec2,
    scale_factor: f32,
    node: &ComputedNode,
    transform: UiGlobalTransform,
) -> bool {
    node.contains_point(transform, cursor * scale_factor)
}

fn interact_with_hud_context_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    mut state: ResMut<HudState>,
    mut windows: Query<(Entity, &mut HudWindow, &Visibility)>,
    menus: Query<Entity, With<HudContextMenu>>,
    close_buttons: Query<(&Pressed, &CloseHudWindow), Changed<Pressed>>,
    transient_windows: Query<(), With<TemporaryHudWindow>>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform, &InheritedVisibility), With<UiButton>>,
) {
    state.suppress_world_click =
        menus.iter().next().is_some() && mouse.just_pressed(MouseButton::Left);
    let closed = close_buttons
        .iter()
        .map(|(_, close)| close.0)
        .collect::<Vec<_>>();
    for entity in closed {
        if transient_windows.get(entity).is_ok() {
            commands.entity(entity).despawn();
        } else if let Ok((_, mut hud_window, _)) = windows.get_mut(entity) {
            hud_window.visible = false;
            hud_window.dragging = false;
        }
        state.suppress_world_click = true;
    }

    let (cursor, cursor_scale_factor) = window.map_or((None, 1.0), |window| {
        (window.cursor_position(), window.scale_factor())
    });
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
        let over_button = buttons.iter().any(|(node, transform, visibility)| {
            visibility.get()
                && ui_node_contains_cursor(cursor, cursor_scale_factor, node, *transform)
        });
        if let Some(target) = target.filter(|_| !over_button) {
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
    mut windows: Query<(&mut HudWindow, &mut Visibility)>,
) {
    for (mut window, mut visibility) in &mut windows {
        if window
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
    mut windows: Query<
        (&mut HudWindow, &mut Node, &mut Visibility),
        (Added<HudWindow>, Without<TemporaryHudWindow>),
    >,
) {
    for (mut window, mut node, mut visibility) in &mut windows {
        window.visible = layout.visible(window.layout_type, &window.key, window.visible);
        visibility.set_if_neq(if window.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        if let Some(position) = layout.position(window.layout_type, &window.key) {
            window.position = hud_layout::constrain_panel_position(position, window.size);
            node.left = px(window.position.x);
            node.top = px(window.position.y);
        }
    }
}

fn save_hud_layout(
    windows: Query<&HudWindow, Without<TemporaryHudWindow>>,
    mut layout: ResMut<HudLayout>,
) {
    if windows.iter().any(|window| window.dragging) {
        return;
    }
    let mut changed = false;
    for window in &windows {
        changed |= layout.set_position(window.layout_type, &window.key, window.position);
        changed |= layout.set_visible(window.layout_type, &window.key, window.visible);
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
    layout: Res<HudLayout>,
    mut last_layout: Local<Option<HudLayoutType>>,
    mut grid: Single<&mut Node, With<SnapGrid>>,
    mode: Res<State<crate::combat::GameMode>>,
    mut windows: Query<(&mut HudWindow, &mut Node, &mut Visibility), Without<SnapGrid>>,
) {
    let active_layout = match mode.get() {
        crate::combat::GameMode::Combat => HudLayoutType::Combat,
        crate::combat::GameMode::World => HudLayoutType::World,
    };
    let layout_changed = *last_layout != Some(active_layout);
    *last_layout = Some(active_layout);
    state.cursor_bounds.clear();
    let mut dragging = false;
    for (mut window, mut node, mut visibility) in &mut windows {
        if layout_changed && window.module_type == HudModuleType::Both {
            window.layout_type = active_layout;
            window.visible = layout.visible(active_layout, &window.key, window.visible);
            if let Some(position) = layout.position(active_layout, &window.key) {
                window.position = hud_layout::constrain_panel_position(position, window.size);
            }
        }
        let available =
            window.module_type == HudModuleType::Both || active_layout == HudLayoutType::Combat;
        let active = available && window.visible && window.measured;
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
    fn cursor_hit_testing_uses_computed_ui_node_bounds_and_window_scale() {
        let node = ComputedNode {
            size: Vec2::splat(40.0),
            ..default()
        };
        let transform = UiGlobalTransform::default();

        assert!(ui_node_contains_cursor(Vec2::ZERO, 1.0, &node, transform));
        assert!(ui_node_contains_cursor(
            Vec2::splat(5.0),
            2.0,
            &node,
            transform
        ));
        assert!(ui_node_contains_cursor(
            Vec2::splat(15.0),
            1.0,
            &node,
            transform
        ));
        assert!(!ui_node_contains_cursor(
            Vec2::splat(15.0),
            2.0,
            &node,
            transform
        ));
    }

    #[test]
    fn stat_categories_use_the_requested_semantic_colors() {
        assert_eq!(stat_color(StatCategory::Hp), Color::srgb(0.92, 0.24, 0.28));
        assert_eq!(stat_color(StatCategory::Mp), Color::srgb(0.30, 0.82, 0.42));
        assert_eq!(stat_color(StatCategory::Ap), Color::srgb(0.30, 0.58, 0.96));
        assert_eq!(
            stat_color(StatCategory::Physical),
            Color::srgb(0.95, 0.58, 0.20)
        );
        assert_eq!(
            stat_color(StatCategory::Magical),
            Color::srgb(0.70, 0.40, 0.92)
        );
    }

    #[test]
    fn hud_layout_round_trips_named_positions() {
        let mut layout = HudLayout::default();
        assert!(layout.set_position(HudLayoutType::World, STATS_KEY, Vec2::new(48.0, 64.0)));
        assert!(!layout.set_position(HudLayoutType::World, STATS_KEY, Vec2::new(48.0, 64.0)));

        let encoded = serde_json::to_string(&layout).unwrap();
        let decoded: HudLayout = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            decoded.position(HudLayoutType::World, STATS_KEY),
            Some(Vec2::new(48.0, 64.0))
        );
        assert!(decoded.visible(HudLayoutType::World, STATS_KEY, true));
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
    fn combat_and_world_layouts_keep_independent_positions_and_visibility() {
        let mut layout = HudLayout::default();
        layout.set_position(HudLayoutType::Combat, STATS_KEY, Vec2::new(16.0, 32.0));
        layout.set_position(HudLayoutType::World, STATS_KEY, Vec2::new(96.0, 112.0));
        layout.set_visible(HudLayoutType::Combat, STATS_KEY, false);

        let encoded = serde_json::to_string(&layout).unwrap();
        let decoded: HudLayout = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            decoded.position(HudLayoutType::Combat, STATS_KEY),
            Some(Vec2::new(16.0, 32.0))
        );
        assert_eq!(
            decoded.position(HudLayoutType::World, STATS_KEY),
            Some(Vec2::new(96.0, 112.0))
        );
        assert!(!decoded.visible(HudLayoutType::Combat, STATS_KEY, true));
        assert!(decoded.visible(HudLayoutType::World, STATS_KEY, true));
    }

    #[test]
    fn hud_layout_round_trips_closed_visibility_and_loads_legacy_layouts() {
        let mut layout = HudLayout::default();
        assert!(layout.set_visible(HudLayoutType::World, STATS_KEY, false));
        assert!(!layout.set_visible(HudLayoutType::World, STATS_KEY, false));

        let encoded = serde_json::to_string(&layout).unwrap();
        let decoded: HudLayout = serde_json::from_str(&encoded).unwrap();
        assert!(!decoded.visible(HudLayoutType::World, STATS_KEY, true));

        let mut legacy: HudLayout =
            serde_json::from_str(r#"{"positions":{"Stats":[48.0,64.0]}}"#).unwrap();
        legacy.migrate_legacy();
        assert_eq!(
            legacy.position(HudLayoutType::Combat, STATS_KEY),
            Some(Vec2::new(48.0, 64.0))
        );
        assert_eq!(
            legacy.position(HudLayoutType::World, STATS_KEY),
            Some(Vec2::new(48.0, 64.0))
        );
        assert!(!legacy.visible(HudLayoutType::World, "Inventory", false));
    }
}

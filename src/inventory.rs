use crate::{hud, hud_layout::HudGrid};
use bevy::{prelude::*, ui::Pressed, ui_widgets::Button as UiButton};
use serde::{Deserialize, Serialize};

const INVENTORY_KEY: &str = "Inventory";
const EQUIPMENT_KEY: &str = "Equipment";
const ABILITY_CELL_SIZE: f32 = 36.0;
const EQUIPMENT_CELL_SIZE: f32 = ABILITY_CELL_SIZE * 2.0;
const DOUBLE_CLICK_SECS: f32 = 0.35;
const EQUIPMENT_PATH: &str = "assets/equipment.json";
const EQUIPPED_ITEMS_PATH: &str = "equipped_items.json";

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct EquipmentPanel;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(super) enum EquipmentType {
    Hat,
    Cape,
    Belt,
    Boots,
}

impl EquipmentType {
    fn letter(self) -> &'static str {
        match self {
            Self::Hat => "H",
            Self::Cape => "C",
            Self::Belt => "B",
            Self::Boots => "T",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Hat => Color::srgb(0.48, 0.28, 0.72),
            Self::Cape => Color::srgb(0.78, 0.22, 0.30),
            Self::Belt => Color::srgb(0.65, 0.43, 0.18),
            Self::Boots => Color::srgb(0.20, 0.52, 0.58),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
pub(super) struct EquipmentModifiers {
    #[serde(default)]
    pub(super) max_hp: i32,
    #[serde(default)]
    pub(super) mp: i32,
    #[serde(default)]
    pub(super) ap: i32,
    #[serde(default)]
    pub(super) physical_damage: i32,
    #[serde(default)]
    pub(super) magical_damage: i32,
}

impl std::ops::AddAssign for EquipmentModifiers {
    fn add_assign(&mut self, rhs: Self) {
        self.max_hp += rhs.max_hp;
        self.mp += rhs.mp;
        self.ap += rhs.ap;
        self.physical_damage += rhs.physical_damage;
        self.magical_damage += rhs.magical_damage;
    }
}

#[derive(Clone, Debug, Deserialize)]
struct EquipmentItem {
    name: String,
    kind: EquipmentType,
    letter: String,
    modifiers: EquipmentModifiers,
}

#[derive(Resource, Default)]
struct EquipmentCatalog(Vec<EquipmentItem>);

#[derive(Resource, Default)]
struct Equipped([Option<usize>; 4]);

#[derive(Deserialize, Serialize)]
struct SavedEquipped([Option<String>; 4]);

#[derive(Resource, Default)]
pub(super) struct CurrentEquipmentModifiers(pub(super) EquipmentModifiers);

impl Equipped {
    fn slot(kind: EquipmentType) -> usize {
        match kind {
            EquipmentType::Hat => 0,
            EquipmentType::Cape => 1,
            EquipmentType::Belt => 2,
            EquipmentType::Boots => 3,
        }
    }

    fn modifiers(&self, catalog: &EquipmentCatalog) -> EquipmentModifiers {
        self.0
            .iter()
            .flatten()
            .fold(EquipmentModifiers::default(), |mut total, index| {
                if let Some(item) = catalog.0.get(*index) {
                    total += item.modifiers;
                }
                total
            })
    }
}

#[derive(Component)]
struct InventoryItemButton(usize);

#[derive(Component)]
struct InventoryItemLabel(usize);

#[derive(Component)]
struct EquippedItemIcon(usize);

#[derive(Component)]
struct EquipmentSlotView;

#[derive(Component)]
struct EquipmentItemContextMenu;

#[derive(Component)]
struct EquipmentItemInfoAction(usize);

pub(super) struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EquipmentCatalog>()
            .init_resource::<Equipped>()
            .init_resource::<CurrentEquipmentModifiers>()
            .add_systems(
                Startup,
                (load_equipment, load_equipped, spawn_inventory_panels).chain(),
            )
            .add_systems(
                Update,
                (
                    equip_on_double_click,
                    unequip_on_double_click,
                    open_equipment_item_context_menu,
                    select_equipment_item_info,
                    save_equipped,
                    sync_equipment_views,
                ),
            );
    }
}

fn load_equipment(mut catalog: ResMut<EquipmentCatalog>) {
    let contents = std::fs::read_to_string(EQUIPMENT_PATH)
        .unwrap_or_else(|error| panic!("failed to read {EQUIPMENT_PATH}: {error}"));
    catalog.0 = serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("{EQUIPMENT_PATH} must contain valid equipment: {error}"));
    assert_eq!(
        catalog.0.len(),
        8,
        "equipment.json must define exactly eight items"
    );
}

fn load_equipped(mut equipped: ResMut<Equipped>, catalog: Res<EquipmentCatalog>) {
    let contents = match std::fs::read_to_string(EQUIPPED_ITEMS_PATH) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            warn!("Could not read {EQUIPPED_ITEMS_PATH}: {error}");
            return;
        }
    };
    let saved = match serde_json::from_str::<SavedEquipped>(&contents) {
        Ok(saved) => saved,
        Err(error) => {
            warn!("Could not parse {EQUIPPED_ITEMS_PATH}: {error}");
            return;
        }
    };
    for (slot, saved_name) in saved.0.into_iter().enumerate() {
        equipped.0[slot] = saved_name.and_then(|name| {
            catalog
                .0
                .iter()
                .position(|item| item.name == name && Equipped::slot(item.kind) == slot)
        });
    }
}

fn save_equipped(equipped: Res<Equipped>, catalog: Res<EquipmentCatalog>) {
    if !equipped.is_changed() {
        return;
    }
    let saved = SavedEquipped(std::array::from_fn(|slot| {
        equipped.0[slot].and_then(|index| catalog.0.get(index).map(|item| item.name.clone()))
    }));
    match serde_json::to_string_pretty(&saved)
        .map_err(std::io::Error::other)
        .and_then(|contents| std::fs::write(EQUIPPED_ITEMS_PATH, contents))
    {
        Ok(()) => {}
        Err(error) => warn!("Could not save {EQUIPPED_ITEMS_PATH}: {error}"),
    }
}

fn spawn_inventory_panels(
    mut commands: Commands,
    layout: Res<hud::HudLayout>,
    catalog: Res<EquipmentCatalog>,
) {
    let inventory_grid = HudGrid::fixed_cells(5, 10, ABILITY_CELL_SIZE, 8.0, UiRect::all(px(8.0)));
    let (_, inventory_content) = hud::spawn_hud_window(
        &mut commands,
        INVENTORY_KEY,
        Some(KeyCode::KeyI),
        InventoryPanel,
        layout
            .position(hud::HudLayoutType::World, INVENTORY_KEY)
            .unwrap_or(Vec2::new(560.0, 128.0)),
        inventory_grid,
        hud::HudModuleType::Both,
        false,
    );
    commands
        .entity(inventory_content)
        .with_children(|inventory| {
            for (index, item) in catalog.0.iter().enumerate() {
                inventory
                    .spawn((
                        InventoryItemButton(index),
                        Name::new(item.name.clone()),
                        UiButton,
                        Node {
                            width: px(ABILITY_CELL_SIZE),
                            height: px(ABILITY_CELL_SIZE),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(2.0)),
                            ..default()
                        },
                        BackgroundColor(item.kind.color()),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                    ))
                    .with_children(|icon| {
                        icon.spawn((
                            InventoryItemLabel(index),
                            Text::new(item.letter.clone()),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });
            }
            for _ in catalog.0.len()..50 {
                empty_slot(inventory, ABILITY_CELL_SIZE);
            }
        });

    let equipment_grid = HudGrid::from_tracks(
        vec![GridTrack::px(EQUIPMENT_CELL_SIZE)],
        (0..4).map(|_| GridTrack::px(EQUIPMENT_CELL_SIZE)).collect(),
        0.0,
        8.0,
        UiRect::all(px(8.0)),
    );
    let (_, equipment_content) = hud::spawn_hud_window(
        &mut commands,
        EQUIPMENT_KEY,
        Some(KeyCode::KeyE),
        EquipmentPanel,
        layout
            .position(hud::HudLayoutType::World, EQUIPMENT_KEY)
            .unwrap_or(Vec2::new(832.0, 16.0)),
        equipment_grid,
        hud::HudModuleType::Both,
        false,
    );
    commands
        .entity(equipment_content)
        .with_children(|equipment| {
            for kind in [
                EquipmentType::Hat,
                EquipmentType::Cape,
                EquipmentType::Belt,
                EquipmentType::Boots,
            ] {
                equipment
                    .spawn((
                        EquipmentSlotView,
                        Node {
                            width: px(EQUIPMENT_CELL_SIZE),
                            height: px(EQUIPMENT_CELL_SIZE),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.12, 0.15, 0.20, 0.72)),
                        BorderColor::all(Color::srgb(0.30, 0.40, 0.52)),
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            Text::new(kind.letter()),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
                            Pickable::IGNORE,
                        ));
                        for (index, item) in catalog
                            .0
                            .iter()
                            .enumerate()
                            .filter(|(_, item)| item.kind == kind)
                        {
                            slot.spawn((
                                EquippedItemIcon(index),
                                UiButton,
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(0),
                                    top: px(0),
                                    width: px(EQUIPMENT_CELL_SIZE),
                                    height: px(EQUIPMENT_CELL_SIZE),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(item.kind.color()),
                                Visibility::Hidden,
                            ))
                            .with_children(|icon| {
                                icon.spawn((
                                    Text::new(item.letter.clone()),
                                    TextFont {
                                        font_size: FontSize::Px(22.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    Pickable::IGNORE,
                                ));
                            });
                        }
                    });
            }
        });
}

fn empty_slot(parent: &mut ChildSpawnerCommands, size: f32) {
    parent.spawn((
        Node {
            width: px(size),
            height: px(size),
            border: UiRect::all(px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.15, 0.20, 0.72)),
        BorderColor::all(Color::srgb(0.30, 0.40, 0.52)),
        Pickable::IGNORE,
    ));
}

fn equip_on_double_click(
    time: Res<Time>,
    catalog: Res<EquipmentCatalog>,
    mut equipped: ResMut<Equipped>,
    items: Query<(&Pressed, &InventoryItemButton), Changed<Pressed>>,
    mut last_click: Local<Option<(usize, f32)>>,
) {
    for (_, InventoryItemButton(index)) in &items {
        let now = time.elapsed_secs();
        if last_click
            .is_some_and(|(previous, at)| previous == *index && now - at <= DOUBLE_CLICK_SECS)
        {
            if let Some(item) = catalog.0.get(*index) {
                equipped.0[Equipped::slot(item.kind)] = Some(*index);
            }
            *last_click = None;
        } else {
            *last_click = Some((*index, now));
        }
    }
}

fn unequip_on_double_click(
    time: Res<Time>,
    mut equipped: ResMut<Equipped>,
    icons: Query<(&Pressed, &EquippedItemIcon), Changed<Pressed>>,
    mut last_click: Local<Option<(usize, f32)>>,
) {
    for (_, EquippedItemIcon(index)) in &icons {
        let now = time.elapsed_secs();
        if last_click
            .is_some_and(|(previous, at)| previous == *index && now - at <= DOUBLE_CLICK_SECS)
        {
            let Some(slot) = equipped.0.iter_mut().find(|slot| **slot == Some(*index)) else {
                *last_click = None;
                continue;
            };
            *slot = None;
            *last_click = None;
        } else {
            *last_click = Some((*index, now));
        }
    }
}

fn open_equipment_item_context_menu(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
    items: Query<(
        &InventoryItemButton,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )>,
    equipped_icons: Query<(
        &EquippedItemIcon,
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
    )>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }
    let Some((cursor, scale_factor)) = window.and_then(|window| {
        window
            .cursor_position()
            .map(|cursor| (cursor, window.scale_factor()))
    }) else {
        return;
    };
    let index = items
        .iter()
        .find_map(
            |(InventoryItemButton(index), node, transform, visibility)| {
                (visibility.get()
                    && hud::ui_node_contains_cursor(cursor, scale_factor, node, *transform))
                .then_some(*index)
            },
        )
        .or_else(|| {
            equipped_icons.iter().find_map(
                |(EquippedItemIcon(index), node, transform, visibility)| {
                    (visibility.get()
                        && hud::ui_node_contains_cursor(cursor, scale_factor, node, *transform))
                    .then_some(*index)
                },
            )
        });
    let Some(index) = index else {
        return;
    };
    let position = crate::hud_layout::constrain_panel_position(cursor, Vec2::new(112.0, 36.0));
    commands
        .spawn((
            EquipmentItemContextMenu,
            hud::HudContextMenu,
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
                EquipmentItemInfoAction(index),
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

fn select_equipment_item_info(
    mut commands: Commands,
    catalog: Res<EquipmentCatalog>,
    actions: Query<(&Pressed, &EquipmentItemInfoAction), Changed<Pressed>>,
    window: Option<Single<&Window, With<bevy::window::PrimaryWindow>>>,
) {
    let Some((_, action)) = actions.iter().next() else {
        return;
    };
    let index = action.0;
    let Some(item) = catalog.0.get(index) else {
        return;
    };
    let position = window
        .and_then(|window| window.cursor_position())
        .unwrap_or(Vec2::new(400.0, 200.0))
        + Vec2::new(16.0, 16.0);
    let grid = HudGrid::content_sized(1, 6, 16.0, 8.0, UiRect::all(px(10.0)));
    let (_, content) = hud::spawn_hud_window(
        &mut commands,
        &format!("{} info", item.name),
        None,
        hud::TemporaryHudWindow,
        position,
        grid,
        hud::HudModuleType::Both,
        true,
    );
    commands.entity(content).with_children(|body| {
        body.spawn((
            Text::new(format!("{} - {:?}", item.name, item.kind)),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
        for (line, category) in [
            (
                format!("Total HP {:+}", item.modifiers.max_hp),
                hud::StatCategory::Hp,
            ),
            (
                format!("Total MP {:+}", item.modifiers.mp),
                hud::StatCategory::Mp,
            ),
            (
                format!("Total AP {:+}", item.modifiers.ap),
                hud::StatCategory::Ap,
            ),
            (
                format!("Physical Damage {:+}%", item.modifiers.physical_damage),
                hud::StatCategory::Physical,
            ),
            (
                format!("Magical Damage {:+}%", item.modifiers.magical_damage),
                hud::StatCategory::Magical,
            ),
        ] {
            body.spawn((
                Text::new(line),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(hud::stat_color(category)),
            ));
        }
    });
}

fn sync_equipment_views(
    catalog: Res<EquipmentCatalog>,
    equipped: Res<Equipped>,
    mut inventory_items: Query<(&InventoryItemButton, &mut BackgroundColor, &mut BorderColor)>,
    mut inventory_labels: Query<(&InventoryItemLabel, &mut Visibility), Without<EquippedItemIcon>>,
    mut icons: Query<
        (&EquippedItemIcon, &mut Visibility),
        (Without<InventoryItemButton>, Without<InventoryItemLabel>),
    >,
    mut current_modifiers: ResMut<CurrentEquipmentModifiers>,
) {
    let modifiers = equipped.modifiers(&catalog);
    if current_modifiers.0 != modifiers {
        current_modifiers.0 = modifiers;
    }
    if !catalog.is_changed() && !equipped.is_changed() {
        return;
    }
    for (InventoryItemButton(index), mut background, mut border) in &mut inventory_items {
        let is_equipped = equipped
            .0
            .iter()
            .flatten()
            .any(|equipped| equipped == index);
        background.0 = if is_equipped {
            Color::srgba(0.12, 0.15, 0.20, 0.72)
        } else {
            catalog.0[*index].kind.color()
        };
        border.set_all(if is_equipped {
            Color::srgb(0.30, 0.40, 0.52)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.35)
        });
    }
    for (InventoryItemLabel(index), mut visibility) in &mut inventory_labels {
        let is_equipped = equipped
            .0
            .iter()
            .flatten()
            .any(|equipped| equipped == index);
        *visibility = if is_equipped {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for (EquippedItemIcon(index), mut visibility) in &mut icons {
        let item = &catalog.0[*index];
        *visibility = if equipped.0[Equipped::slot(item.kind)] == Some(*index) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_catalog() -> EquipmentCatalog {
        EquipmentCatalog(
            serde_json::from_str(&std::fs::read_to_string(EQUIPMENT_PATH).unwrap()).unwrap(),
        )
    }

    #[test]
    fn equipment_catalog_has_two_items_per_slot_with_modifiers() {
        let catalog = load_catalog();
        assert_eq!(catalog.0.len(), 8);
        for kind in [
            EquipmentType::Hat,
            EquipmentType::Cape,
            EquipmentType::Belt,
            EquipmentType::Boots,
        ] {
            assert_eq!(catalog.0.iter().filter(|item| item.kind == kind).count(), 2);
        }
        assert!(catalog.0.iter().all(|item| {
            item.modifiers.max_hp != 0
                || item.modifiers.mp != 0
                || item.modifiers.ap != 0
                || item.modifiers.physical_damage != 0
                || item.modifiers.magical_damage != 0
        }));
    }

    #[test]
    fn replacing_a_slot_replaces_its_modifiers_instead_of_stacking() {
        let catalog = load_catalog();
        let first_hat = catalog
            .0
            .iter()
            .position(|item| item.kind == EquipmentType::Hat)
            .unwrap();
        let second_hat = catalog
            .0
            .iter()
            .rposition(|item| item.kind == EquipmentType::Hat)
            .unwrap();
        let mut equipped = Equipped::default();
        equipped.0[Equipped::slot(EquipmentType::Hat)] = Some(first_hat);
        let first = equipped.modifiers(&catalog);
        equipped.0[Equipped::slot(EquipmentType::Hat)] = Some(second_hat);
        let replaced = equipped.modifiers(&catalog);
        assert_eq!(replaced, catalog.0[second_hat].modifiers);
        assert_ne!(first, replaced);
    }
}

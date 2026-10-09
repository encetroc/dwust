use crate::{hud, hud_layout::HudGrid};
use bevy::prelude::*;

const INVENTORY_KEY: &str = "Inventory";
const EQUIPMENT_KEY: &str = "Equipment";
const ABILITY_CELL_SIZE: f32 = 36.0;
const EQUIPMENT_CELL_SIZE: f32 = ABILITY_CELL_SIZE * 2.0;

#[derive(Component)]
struct InventoryPanel;

#[derive(Component)]
struct EquipmentPanel;

pub(super) struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_inventory_panels);
    }
}

fn spawn_inventory_panels(mut commands: Commands, layout: Res<hud::HudLayout>) {
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
            for _ in 0..50 {
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
            for _ in 0..4 {
                empty_slot(equipment, EQUIPMENT_CELL_SIZE);
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

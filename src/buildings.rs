//! Static box placeholders, anchored to the world rather than the moving floor.
use super::*;

#[derive(Component)]
pub(super) struct Building(pub(super) Vec3);

pub(super) fn spawn_buildings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viewport: Res<GameViewport>,
) {
    let half_extents = screen_half_extents(viewport.size);
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 0.57, 0.46),
        perceptual_roughness: 0.95,
        ..default()
    });
    for screen in [
        IVec2::ZERO,
        IVec2::new(-1, 0),
        IVec2::new(1, 0),
        IVec2::new(0, -1),
        IVec2::new(0, 1),
    ] {
        let center = SCREEN_RIGHT * (screen.x as f32 * half_extents.x * 2.0)
            - SCREEN_UP * (screen.y as f32 * half_extents.y * 2.0);
        let layout: &[(Vec2, Vec3)] = if screen == IVec2::ZERO {
            &STARTING_BUILDINGS
        } else {
            &NEIGHBOR_BUILDINGS
        };
        for &(offset, size) in layout {
            let position = center
                + SCREEN_RIGHT * (offset.x * half_extents.x)
                + SCREEN_UP * (offset.y * half_extents.y)
                + Vec3::Y * (size.y / 2.0);
            commands.spawn((
                Building(size),
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(position),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_screen_has_three_boxes_and_each_neighbor_has_two() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<GameViewport>()
            .add_systems(Startup, spawn_buildings);
        app.update();

        let half_extents = screen_half_extents(app.world().resource::<GameViewport>().size);
        let mut counts = std::collections::HashMap::new();
        let mut query = app
            .world_mut()
            .query_filtered::<&Transform, With<Building>>();
        for transform in query.iter(app.world()) {
            let position = transform.translation;
            let screen = IVec2::new(
                (position.dot(SCREEN_RIGHT) / (half_extents.x * 2.0)).round() as i32,
                (-position.dot(SCREEN_UP) / (half_extents.y * 2.0)).round() as i32,
            );
            *counts.entry(screen).or_insert(0) += 1;
            assert!(position.y > 0.0);
        }
        assert_eq!(counts.len(), 5);
        assert_eq!(counts[&IVec2::ZERO], 3);
        for screen in [IVec2::NEG_X, IVec2::X, IVec2::NEG_Y, IVec2::Y] {
            assert_eq!(counts[&screen], 2);
        }
    }
}

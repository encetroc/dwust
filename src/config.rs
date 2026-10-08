//! Central game settings. Edit these defaults and restart the game.
use bevy::prelude::*;

pub const SCREEN_ANIMATION_ENABLED: bool = true;
/// Seconds; zero disables animation too. Uses smooth ease-in/ease-out.
pub const SCREEN_ANIMATION_DURATION: f32 = 0.18;

pub const WORLD_SIZE: f32 = 128.0;
pub const TILE_COUNT: u32 = 64;
pub const PLAYER_RADIUS: f32 = 0.4;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const VIEW_HEIGHT: f32 = 32.0;
pub const DEFAULT_MOVE_SPEED: f32 = 14.0;
/// (ground offset as a fraction of screen half-extents, box dimensions).
/// Keep the central horizontal and vertical travel paths clear.
pub const STARTING_BUILDINGS: [(Vec2, Vec3); 3] = [
    (Vec2::new(-0.5, 0.35), Vec3::new(6.0, 5.0, 4.0)),
    (Vec2::new(0.45, 0.3), Vec3::new(4.5, 4.5, 4.5)),
    (Vec2::new(-0.4, -0.4), Vec3::new(7.0, 3.5, 4.0)),
];
pub const NEIGHBOR_BUILDINGS: [(Vec2, Vec3); 2] = [
    (Vec2::new(-0.45, 0.3), Vec3::new(5.0, 4.0, 7.0)),
    (Vec2::new(0.45, -0.35), Vec3::new(5.0, 5.0, 5.0)),
];
pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 720;
// Leave space for the capsule's full height and the edge overlays.
pub const SCREEN_MARGIN: Vec2 = Vec2::new(4.0, 10.0);
pub const SCREEN_RIGHT: Vec3 = Vec3::new(
    std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
);
pub const SCREEN_UP: Vec3 = Vec3::new(
    -std::f32::consts::FRAC_1_SQRT_2,
    0.0,
    -std::f32::consts::FRAC_1_SQRT_2,
);

#[derive(Resource)]
pub struct MovementSettings {
    pub speed: f32,
}

impl Default for MovementSettings {
    fn default() -> Self {
        Self {
            speed: configured_speed(std::env::var("DWUST_MOVE_SPEED").ok().as_deref()),
        }
    }
}

pub fn configured_speed(value: Option<&str>) -> f32 {
    value
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|speed| speed.is_finite() && (2.0..=60.0).contains(speed))
        .unwrap_or(DEFAULT_MOVE_SPEED)
}

#[derive(Resource)]
pub struct ScreenAnimationSettings {
    pub enabled: bool,
    pub duration: f32,
}

impl Default for ScreenAnimationSettings {
    fn default() -> Self {
        Self {
            enabled: SCREEN_ANIMATION_ENABLED,
            duration: SCREEN_ANIMATION_DURATION,
        }
    }
}

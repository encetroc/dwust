//! Windowless playtest of the real GamePlugin, including image-target UI picking.
use super::*;
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    asset::uuid::Uuid,
    camera::RenderTarget,
    input::InputSystems,
    picking::{
        PickingSystems,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use std::{path::PathBuf, time::Duration};

const POINTER: PointerId = PointerId::Custom(Uuid::from_u128(0x4457555354));
const CHECKPOINTS: [&str; 12] = [
    "initial",
    "movement",
    "coordinates-only",
    "key-right",
    "key-up",
    "key-left",
    "key-down",
    "click-right",
    "click-down",
    "click-left",
    "click-up",
    "returned-origin",
];

#[derive(Resource)]
struct Verification {
    output: PathBuf,
    target: Option<Handle<Image>>,
    stage: usize,
    elapsed: u32,
    total_frames: u32,
    pending: bool,
    captures: usize,
    start: Vec3,
}

pub(super) fn run(output: String) {
    let output = PathBuf::from(output);
    assert!(
        !output.exists(),
        "use a fresh output directory to avoid stale evidence"
    );
    std::fs::create_dir_all(&output).expect("create verification output");
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<WinitPlugin>(),
        )
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(MovementSettings {
            speed: DEFAULT_MOVE_SPEED,
        })
        .insert_resource(Verification {
            output,
            target: None,
            stage: 0,
            elapsed: 0,
            total_frames: 0,
            pending: false,
            captures: 0,
            start: Vec3::new(0.0, PLAYER_HEIGHT / 2.0, 0.0),
        })
        .add_plugins(GamePlugin)
        .add_systems(Startup, configure_target.after(setup).after(setup_overlay))
        .add_systems(First, script_pointer.in_set(PickingSystems::Input))
        .add_systems(
            PreUpdate,
            script_buttons.after(InputSystems).after(sync_viewport),
        )
        .add_systems(PostUpdate, checkpoint)
        .run();
}

fn configure_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Camera3d>>,
    roots: Query<Entity, (With<Node>, Without<ChildOf>)>,
    mut verification: ResMut<Verification>,
) {
    let target = images.add(Image::new_target_texture(
        1280,
        720,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    commands
        .entity(*camera)
        .insert(RenderTarget::Image(target.clone().into()));
    for root in &roots {
        commands.entity(root).insert(UiTargetCamera(*camera));
    }
    commands.spawn(POINTER);
    verification.target = Some(target);
}

fn pointer_position(stage: usize) -> Option<Vec2> {
    match stage {
        1 => Some(Vec2::new(820.0, 360.0)),
        7 => Some(Vec2::new(1240.0, 360.0)),
        8 => Some(Vec2::new(640.0, 684.0)),
        9 => Some(Vec2::new(40.0, 360.0)),
        10 => Some(Vec2::new(640.0, 36.0)),
        _ => None,
    }
}

fn stage_key(stage: usize) -> Option<KeyCode> {
    match stage {
        3 => Some(KeyCode::ArrowRight),
        4 => Some(KeyCode::ArrowUp),
        5 => Some(KeyCode::ArrowLeft),
        6 => Some(KeyCode::ArrowDown),
        _ => None,
    }
}

fn script_pointer(mut verification: ResMut<Verification>, mut inputs: MessageWriter<PointerInput>) {
    verification.total_frames += 1;
    assert!(
        verification.total_frames < 4000,
        "verification frame watchdog expired"
    );
    if verification.pending || verification.stage >= CHECKPOINTS.len() {
        return;
    }
    verification.elapsed += 1;
    let Some(target) = &verification.target else {
        return;
    };
    let position = pointer_position(verification.stage).unwrap_or(Vec2::new(640.0, 360.0));
    let action = match verification.elapsed {
        1 => PointerAction::Move { delta: Vec2::ZERO },
        2 if pointer_position(verification.stage).is_some() => {
            PointerAction::Press(PointerButton::Primary)
        }
        3 if pointer_position(verification.stage).is_some() => {
            PointerAction::Release(PointerButton::Primary)
        }
        _ => return,
    };
    inputs.write(PointerInput::new(
        POINTER,
        Location {
            target: RenderTarget::Image(target.clone().into())
                .normalize(None)
                .unwrap(),
            position,
        },
        action,
    ));
}

fn script_buttons(
    verification: Res<Verification>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut viewport: ResMut<GameViewport>,
) {
    if verification.pending || verification.stage >= CHECKPOINTS.len() {
        return;
    }
    viewport.cursor = pointer_position(verification.stage);
    if let Some(key) = stage_key(verification.stage) {
        match verification.elapsed {
            1 => keyboard.press(key),
            2 => keyboard.release(key),
            _ => (),
        }
    }
    if pointer_position(verification.stage).is_some() {
        match verification.elapsed {
            2 => mouse.press(MouseButton::Left),
            3 => mouse.release(MouseButton::Left),
            _ => (),
        }
    }
}

fn checkpoint(
    mut commands: Commands,
    mut verification: ResMut<Verification>,
    screen: Res<WorldScreen>,
    settings: Res<MovementSettings>,
    player: Single<(&Transform, &Player)>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
) {
    assert!(windows.is_empty(), "headless mode created a Window entity");
    if verification.pending || verification.stage >= CHECKPOINTS.len() {
        return;
    }
    if verification.stage == 1 && verification.elapsed == 2 {
        let distance = player.0.translation.distance(verification.start);
        assert!(
            (distance - DEFAULT_MOVE_SPEED / 60.0).abs() < 0.0001,
            "wrong movement speed per tick: {distance}"
        );
    }
    if (7..=10).contains(&verification.stage) && verification.elapsed == 2 {
        let previous = match verification.stage {
            8 => IVec2::new(1, 0),
            9 => IVec2::new(1, 1),
            10 => IVec2::new(0, 1),
            _ => IVec2::ZERO,
        };
        if screen.coordinates == previous {
            assert!(player.1.travel.is_some(), "arrow did not start travel");
            assert!(
                player.1.destination.is_some(),
                "arrow did not start walking toward its nearest edge point"
            );
        } else {
            let expected = match verification.stage {
                7 => IVec2::new(1, 0),
                8 => IVec2::new(1, 1),
                9 => IVec2::new(0, 1),
                _ => IVec2::ZERO,
            };
            assert_eq!(
                screen.coordinates, expected,
                "instant transition is valid only when already at the nearest edge point"
            );
        }
    }
    let wait = match verification.stage {
        0 => 120,
        1 => 90,
        3..=10 => 300,
        _ => 20,
    };
    if verification.elapsed < wait {
        return;
    }
    let (transform, player) = *player;
    assert!(
        player.destination.is_none(),
        "movement did not finish or UI click leaked to ground"
    );
    // Each keyboard/UI journey makes a loop back to [0, 0].
    let expected = match verification.stage {
        3 | 7 => IVec2::new(1, 0),
        4 => IVec2::new(1, -1),
        5 => IVec2::new(0, -1),
        8 => IVec2::new(1, 1),
        9 => IVec2::new(0, 1),
        _ => IVec2::ZERO,
    };
    assert_eq!(
        screen.coordinates, expected,
        "{}: screen mismatch",
        CHECKPOINTS[verification.stage]
    );
    if verification.stage == 1 {
        let distance = transform.translation.distance(verification.start);
        assert!(
            (distance - 8.0).abs() < 0.05,
            "ground click movement distance: {distance}"
        );
    }
    assert_eq!(settings.speed, DEFAULT_MOVE_SPEED);
    let (camera, global) = *camera;
    let capsule_center = camera
        .world_to_viewport(global, transform.translation)
        .expect("player projects to viewport");
    assert!(
        (0.0..1280.0).contains(&capsule_center.x) && (0.0..720.0).contains(&capsule_center.y),
        "player entered outside viewport: {capsule_center}"
    );
    let stage = verification.stage;
    let name = CHECKPOINTS[stage];
    let output = verification.output.clone();
    println!(
        "ASSERT {name}: screen {:?}, speed {}, player {:?}, frame {}",
        screen.coordinates, settings.speed, transform.translation, verification.total_frames
    );
    verification.pending = true;
    commands.spawn(Screenshot::image(verification.target.as_ref().unwrap().clone())).observe(
        move |event: On<ScreenshotCaptured>, mut verification: ResMut<Verification>, mut exit: MessageWriter<AppExit>| {
            assert_eq!(verification.stage, stage);
            let image = event.image.clone().try_into_dynamic().expect("decode checkpoint").to_rgb8();
            assert_eq!(image.dimensions(), (1280, 720));
            let blue_pixels = image.pixels().filter(|pixel| pixel[2] > 100 && pixel[2] > pixel[0].saturating_add(30) && pixel[2] > pixel[1].saturating_add(20)).count();
            assert!(blue_pixels > 80, "player missing from capture {name}: {blue_pixels} blue pixels");
            image.save(output.join(format!("{stage:02}-{name}.png"))).expect("save checkpoint PNG");
            verification.captures += 1;
            verification.stage += 1;
            verification.elapsed = 0;
            verification.pending = false;
            if verification.captures == CHECKPOINTS.len() {
                let summary = format!("PASS: Bevy 0.19.1, 1280x720, zero windows, ground movement and speed per tick, minimal overlay, four keyboard directions, four overlay nearest-edge travel directions, {} checked captures\n", verification.captures);
                std::fs::write(output.join("result.txt"), &summary).expect("save result");
                println!("{summary}");
                exit.write(AppExit::Success);
            }
        },
    );
}

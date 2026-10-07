use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::RenderTarget,
    input::InputSystems,
    prelude::*,
    render::{render_resource::TextureFormat, view::screenshot::{Screenshot, ScreenshotCaptured}, RenderPlugin},
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use std::time::Duration;

#[derive(Component)]
struct Player;
#[derive(Resource)]
struct Target(Handle<Image>);
#[derive(Resource, Default)]
struct Progress { frame: u32, captures: u32, press_seen: bool, release_seen: bool }

fn main() {
    std::fs::create_dir_all("captures").unwrap();
    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
            .set(RenderPlugin { synchronous_pipeline_compilation: true, ..default() })
            .disable::<WinitPlugin>())
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
        .init_resource::<Progress>()
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, script.after(InputSystems))
        .add_systems(Update, movement)
        .add_systems(PostUpdate, capture)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let target = images.add(Image::new_target_texture(320, 240, TextureFormat::Rgba8UnormSrgb, None));
    commands.spawn((Camera2d, RenderTarget::Image(target.clone().into())));
    commands.insert_resource(Target(target));
    commands.spawn((Player, Sprite::from_color(Color::srgb(0.2, 0.8, 0.3), Vec2::splat(40.0)), Transform::from_xyz(-60.0, 0.0, 0.0)));
}

fn script(mut p: ResMut<Progress>, mut keys: ResMut<ButtonInput<KeyCode>>) {
    p.frame += 1;
    if p.frame == 60 { keys.press(KeyCode::ArrowRight); }
    if p.frame == 120 { keys.release(KeyCode::ArrowRight); }
    assert!(p.frame < 600, "capture watchdog expired");
}

fn movement(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut player: Single<&mut Transform, With<Player>>, mut p: ResMut<Progress>) {
    p.press_seen |= keys.just_pressed(KeyCode::ArrowRight);
    p.release_seen |= keys.just_released(KeyCode::ArrowRight);
    if keys.pressed(KeyCode::ArrowRight) { player.translation.x += 120.0 * time.delta_secs(); }
}

fn capture(mut commands: Commands, target: Res<Target>, p: Res<Progress>, player: Single<&Transform, With<Player>>, windows: Query<&Window>) {
    assert_eq!(windows.iter().count(), 0, "a window was created");
    let name = match p.frame {
        40 => "before",
        140 => {
            assert!((player.translation.x - 60.0).abs() < 0.1, "wrong position: {}", player.translation.x);
            assert!(p.press_seen && p.release_seen, "input edges not consumed");
            "after"
        }
        _ => return,
    };
    commands.spawn(Screenshot::image(target.0.clone())).observe(
        move |event: On<ScreenshotCaptured>, mut p: ResMut<Progress>, mut exit: MessageWriter<AppExit>| {
            let image = event.image.clone().try_into_dynamic().expect("decode capture").to_rgb8();
            assert_eq!(image.dimensions(), (320, 240));
            let green_pixels = image.pixels().filter(|v| v[1] > 120 && v[1] > v[0].saturating_add(30)).count();
            assert!(green_pixels > 100, "empty/incorrect render: {green_pixels} green pixels");
            image.save(format!("captures/{name}.png")).expect("save capture");
            println!("CAPTURE {name}: {} bytes, {green_pixels} green pixels", image.as_raw().len());
            p.captures += 1;
            if p.captures == 2 {
                println!("PASS: no windows, input press/hold/release, movement, two saved captures");
                exit.write(AppExit::Success);
            }
        });
}

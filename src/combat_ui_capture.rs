use super::{GamePlugin, combat, config::*, hud::HudWindow, setup};
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::TextureFormat,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use std::{path::PathBuf, time::Duration};

#[derive(Resource)]
struct CaptureOutput(PathBuf);

#[derive(Resource)]
struct CaptureState {
    output: PathBuf,
    target: Handle<Image>,
    elapsed: u32,
    pending: bool,
}

type UntargetedUiRoot = (With<Node>, Without<ChildOf>, Without<UiTargetCamera>);

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
        .insert_resource(CaptureOutput(output))
        .add_plugins(GamePlugin)
        .add_systems(Startup, attach_target.after(setup))
        .add_systems(Update, attach_new_ui_roots)
        .add_systems(PostUpdate, capture_combat_ui)
        .run();
}

fn attach_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Camera3d>>,
    roots: Query<Entity, (With<Node>, Without<ChildOf>)>,
    mut state: ResMut<NextState<combat::GameMode>>,
    output: Res<CaptureOutput>,
) {
    let target = images.add(Image::new_target_texture(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    commands
        .entity(*camera)
        .insert(RenderTarget::Image(target.clone().into()));
    for root in &roots {
        commands.entity(root).insert(UiTargetCamera(*camera));
    }
    commands.insert_resource(CaptureState {
        output: output.0.clone(),
        target,
        elapsed: 0,
        pending: false,
    });
    state.set(combat::GameMode::Combat);
}

fn attach_new_ui_roots(
    mut commands: Commands,
    camera: Single<Entity, With<Camera3d>>,
    roots: Query<Entity, UntargetedUiRoot>,
) {
    for root in &roots {
        commands.entity(root).insert(UiTargetCamera(*camera));
    }
}

fn capture_combat_ui(
    mut commands: Commands,
    mut state: ResMut<CaptureState>,
    mode: Res<State<combat::GameMode>>,
    windows: Query<&Window>,
    modules: Query<&Node, With<HudWindow>>,
) {
    assert!(windows.is_empty(), "headless mode created a Window entity");
    if *mode.get() != combat::GameMode::Combat || state.pending {
        return;
    }
    state.elapsed += 1;
    if state.elapsed < 60 {
        return;
    }
    assert_eq!(
        modules.iter().count(),
        4,
        "expected all four combat modules including deployment status"
    );
    state.pending = true;
    let output = state.output.clone();
    commands
        .spawn(Screenshot::image(state.target.clone()))
        .observe(move |event: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
            let image = event
                .image
                .clone()
                .try_into_dynamic()
                .expect("decode combat UI screenshot")
                .to_rgb8();
            assert_eq!(image.dimensions(), (WINDOW_WIDTH, WINDOW_HEIGHT));
            image
                .save(output.join("combat-ui.png"))
                .expect("save combat UI screenshot");
            std::fs::write(
                output.join("result.txt"),
                "PASS: real combat scene, headless render target, 4 combat UI modules including deployment status, 1280x720 screenshot\n",
            )
            .expect("save verification result");
            println!("PASS: captured real combat UI with deployment status at 1280x720");
            exit.write(AppExit::Success);
        });
}

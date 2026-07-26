//! Master Thief — window, frame loop, and the headless capture entry point.

use macroquad::prelude::*;
use macroquad_toolkit::capture;
use master_thief::game::Game;
use master_thief::ui;

fn window_conf() -> Conf {
    capture::capture_window_conf(
        "MASTER_THIEF",
        "Master Thief",
        ui::LOGICAL_WIDTH as i32,
        ui::LOGICAL_HEIGHT as i32,
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new().await;

    // Screenshot harness: with MASTER_THIEF_CAPTURE_PATH set, boot into the
    // named scene, render a fixed number of frames, write a PNG, and exit.
    if let Some(config) = capture::CaptureConfig::from_env("MASTER_THIEF") {
        game.set_capture_scene(&config.scene);
        capture::run_capture(&config, |dt| {
            game.update(dt);
            game.draw();
        })
        .await;
        return;
    }

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}

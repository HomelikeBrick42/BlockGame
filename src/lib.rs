mod game;
pub mod texture;

use game::Game;
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;

    let mut game = {
        let window = WindowBuilder::new()
            .with_title("Block Game")
            .with_visible(false)
            .build(&event_loop)
            .unwrap();
        Game::new(window).await?
    };

    let mut frame_start_time = std::time::Instant::now();
    let mut dt = std::time::Duration::ZERO;

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == game.window().id() => {
            elwt.exit();
        }

        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { event, .. },
            window_id,
        } if window_id == game.window().id() && !elwt.exiting() => {
            game.key_event(event);
        }

        Event::WindowEvent {
            event: WindowEvent::Focused(false),
            window_id,
        } if window_id == game.window().id() && !elwt.exiting() => {
            game.lost_focus();
        }

        Event::NewEvents(cause) => {
            if let StartCause::Init = cause {
                game.window().set_visible(true);
            }
            let last_frame_time = frame_start_time;
            frame_start_time = std::time::Instant::now();
            dt = frame_start_time - last_frame_time;
        }

        Event::AboutToWait if !elwt.exiting() => {
            match game.update(dt) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("{error}");
                    eprintln!("{}", error.backtrace());
                    elwt.exit();
                    return;
                }
            }
            game.window().request_redraw();
        }

        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            window_id,
        } if window_id == game.window().id() && !elwt.exiting() => {
            game.resize(size.width, size.height);
        }

        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            window_id,
        } if window_id == game.window().id() && !elwt.exiting() => match game.render() {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{error}");
                eprintln!("{}", error.backtrace());
                elwt.exit();
            }
        },

        _ => {}
    })?;

    Ok(())
}

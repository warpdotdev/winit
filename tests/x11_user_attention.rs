#![cfg(all(target_os = "linux", feature = "x11"))]

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::Duration;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{UserAttentionType, Window, WindowId};

#[derive(Default)]
struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(event_loop.create_window(Window::default_attributes()).unwrap());
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }
}

#[test]
fn requesting_attention_after_x11_disconnect_does_not_panic() {
    let mut server = match Command::new("Xvfb")
        .args(["-displayfd", "1", "-nolisten", "tcp"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(server) => server,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => panic!("failed to start Xvfb: {error}"),
    };

    let mut display = String::new();
    BufReader::new(server.stdout.take().unwrap()).read_line(&mut display).unwrap();
    let mut builder = EventLoop::builder();
    builder.with_x11().with_any_thread(true);
    let mut event_loop = {
        let previous_display = std::env::var_os("DISPLAY");
        std::env::set_var("DISPLAY", format!(":{}", display.trim()));
        let result = builder.build().unwrap();
        if let Some(previous_display) = previous_display {
            std::env::set_var("DISPLAY", previous_display);
        } else {
            std::env::remove_var("DISPLAY");
        }
        result
    };

    let mut app = App::default();
    event_loop.pump_app_events(Some(Duration::ZERO), &mut app);
    let window = app.window.take().unwrap();
    window.request_user_attention(Some(UserAttentionType::Informational));

    server.kill().unwrap();
    server.wait().unwrap();
    window.request_user_attention(Some(UserAttentionType::Informational));
    // Xlib may try to use the dead connection while destroying the window and event loop.

    std::mem::forget(window);
    std::mem::forget(event_loop);
}

use im_state::ImState;
use te_renderer::initial_config::InitialConfiguration;
use winit::{event_loop::{EventLoop, ControlFlow}, window::WindowBuilder, dpi, event::{Event, WindowEvent}};

mod im_state;

enum ActionTaken {
    Send,
    Clear,
}

fn main() {
    let config = InitialConfiguration {
        ..Default::default()
    };
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let wb = WindowBuilder::new()
        .with_title("Carga progresiva")
        .with_inner_size(dpi::LogicalSize::new(config.screen_size.x, config.screen_size.y));

    let window = wb.build(&event_loop)
        .unwrap();

    let mut im_state = pollster::block_on(ImState::new(&window, config.clone()));
    let mut last_render_time = std::time::Instant::now();
    event_loop.run(move |event, window_target| {
        window_target.set_control_flow(ControlFlow::Poll);
        match &event {
            Event::NewEvents(_cause) => (), // TODO
            Event::WindowEvent { window_id, event } if *window_id == window.id() => {
                match event {
                    WindowEvent::Resized(size) => {
                        im_state.resize(size.clone());
                    },
                    WindowEvent::CloseRequested => window_target.exit(),
                    WindowEvent::KeyboardInput { .. } => {im_state.input(&event);}, // TODO
                    WindowEvent::CursorMoved { device_id: _, position: _, .. } => {im_state.input(&event);},
                    WindowEvent::MouseWheel { .. } => {im_state.input(&event);},
                    WindowEvent::MouseInput { .. } => {im_state.input(&event);},
                    WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let dt = now - last_render_time;
                        last_render_time = now;
                        im_state.update(dt);
                        match im_state.render(&window) {
                            Ok(action) => if let Some(action) = action {
                                match action {
                                    ActionTaken::Send => im_state.send(),
                                    ActionTaken::Clear => im_state.clear(),
                                };
                            },
                            // Reconfigure the surface if lost
                            Err(wgpu::SurfaceError::Lost) => im_state.resize(im_state.state.size),
                            // The system is out of memory, we should quit
                            Err(wgpu::SurfaceError::OutOfMemory) => window_target.exit(),
                            // All other errors (Outdated, Timeout) should be resolved by the next frame
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                    _ => (),
                }
            },
            Event::AboutToWait => window.request_redraw(),
            Event::Suspended => window_target.set_control_flow(ControlFlow::Wait),
            _ => () // ignore windowevents that aren't for current window
        }

        im_state.platform.handle_event(im_state.context.io_mut(), &window, &event)
    }).unwrap()
}

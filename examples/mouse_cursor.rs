use window::WindowSettings;
use winit_window::WinitWindow;
use piston::{Events, EventSettings, MouseCursorEvent};

fn main() {
    let mut window: WinitWindow = WindowSettings::new("Winit Window", (640, 480))
        .fullscreen(false)
        .vsync(true)
        .build()
        .unwrap();

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(cur) = e.mouse_cursor_args() {
            println!("{:?}", cur);
        }
    }
}

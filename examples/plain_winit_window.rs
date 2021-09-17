extern crate window;
extern crate winit_window;

use window::{AdvancedWindow, Window, WindowSettings};
use winit_window::WinitWindow;

fn main() {
    let mut window = WinitWindow::new(&WindowSettings::new("Winit Window", (640, 480)).exit_on_esc(true));
    window.show();

    while !window.should_close() {
        let e = match window.poll_event() {
            Some(e) => e,
            None => continue,
        };
    }
}

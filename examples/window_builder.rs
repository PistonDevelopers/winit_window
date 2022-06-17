extern crate window;
extern crate winit_window;

use window::WindowSettings;
use winit_window::WinitWindow;

fn main() {
    let _: WinitWindow = WindowSettings::new("Winit Window", (640, 480))
        .fullscreen(false)
        .vsync(true)
        .build()
        .unwrap();
}

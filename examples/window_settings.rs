extern crate vulkano;
extern crate window;
extern crate winit_window;

use vulkano::instance::{Instance, Version};
use window::WindowSettings;

use winit_window::VulkanoWindow;

fn main() {
    let instance = Instance::new(
        None,
        Version::V1_2,
        &winit_window::required_extensions(),
        None,
    )
    .unwrap();
    let _ = VulkanoWindow::new(instance, &WindowSettings::new("Winit Window", (640, 480)));
}

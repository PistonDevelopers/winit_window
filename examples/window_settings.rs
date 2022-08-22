#[cfg(feature = "use-vulkano")]
extern crate vulkano;
extern crate window;
extern crate winit_window;

#[cfg(feature = "use-vulkano")]
use vulkano::instance::{Instance, Version};
use window::WindowSettings;

#[cfg(feature = "use-vulkano")]
use winit_window::VulkanoWindow;

#[cfg(feature = "use-vulkano")]
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

#[cfg(not(feature = "use-vulkano"))]
fn main() {}

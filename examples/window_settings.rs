#[cfg(feature = "use-vulkano")]
extern crate vulkano;
extern crate window;
extern crate winit_window;

#[cfg(feature = "use-vulkano")]
use vulkano::{
    VulkanLibrary,
    instance::{Instance, InstanceCreateInfo}
};
#[cfg(feature = "use-vulkano")]
use window::WindowSettings;

#[cfg(feature = "use-vulkano")]
use winit_window::VulkanoWindow;

#[cfg(feature = "use-vulkano")]
fn main() {
    let library = VulkanLibrary::new().unwrap();
    let instance_create_info = InstanceCreateInfo {
        enabled_extensions: winit_window::required_extensions(&library),
        ..Default::default()
    };
    let instance = Instance::new(
        library,
        instance_create_info,
    )
    .unwrap();
    let _ = VulkanoWindow::new(instance, &WindowSettings::new("Winit Window", (640, 480)));
}

#[cfg(not(feature = "use-vulkano"))]
fn main() {}

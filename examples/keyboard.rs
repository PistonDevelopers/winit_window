use window::WindowSettings;
use winit_window::{KeyboardIgnoreModifiers, WinitWindow};
use piston::{Events, EventSettings, PressEvent, TextEvent};

fn main() {
    let mut window: WinitWindow = WindowSettings::new("Winit Window", (640, 480))
        .exit_on_esc(true)
        .build()
        .unwrap();
    // Assume that the keyboard layout is standard English ABC.
    window.keyboard_ignore_modifiers = KeyboardIgnoreModifiers::AbcKeyCode;

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(button) = e.press_args() {
            println!("{:?}", button);
        }
        if let Some(s) = e.text_args() {
            println!("{}", s);
        }
    }
}

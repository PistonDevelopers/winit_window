use window::WindowSettings;
use winit_window::{KeyboardIgnoreModifiers, WinitWindow};
use piston::{
    AdvancedWindow,
    Button,
    Events,
    EventSettings,
    Key,
    TouchEvent,
    PressEvent
};

fn main() {
    let mut window: WinitWindow = WindowSettings::new("Winit Window", (640, 480))
        .exit_on_esc(true)
        .build()
        .unwrap();
    // Assume that the keyboard layout is standard English ABC.
    window.keyboard_ignore_modifiers = KeyboardIgnoreModifiers::AbcKeyCode;

    let mut events = Events::new(EventSettings::new());
    let mut capture_cursor = false;
    while let Some(e) = events.next(&mut window) {
        if let Some(touch) = e.touch_args() {
            println!("touch {:?}", touch);
        }
        if let Some(Button::Keyboard(Key::C)) = e.press_args() {
            capture_cursor = !capture_cursor;
            window.set_capture_cursor(capture_cursor);
        }
    }
}

use window::WindowSettings;
use winit_window::{KeyboardIgnoreModifiers, WinitWindow};
use piston::{
    AdvancedWindow,
    Button,
    Events,
    EventSettings,
    Key,
    MouseCursorEvent,
    MouseRelativeEvent,
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
        if let Some(diff) = e.mouse_relative_args() {
            println!("mouse relative {:?}", diff);
        }
        if let Some(pos) = e.mouse_cursor_args() {
            println!("mouse cursor {:?}", pos);
        }
        if let Some(Button::Keyboard(Key::C)) = e.press_args() {
            capture_cursor = !capture_cursor;
            window.set_capture_cursor(capture_cursor);
        }
    }
}

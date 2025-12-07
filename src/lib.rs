//! A [winit](https://crates.io/crates/winit) window back-end for the Piston game engine.

extern crate input;
extern crate window;
extern crate winit;

use std::sync::Arc;

use input::{
    Button, ButtonArgs, ButtonState, CloseArgs, Event, Input, Key, Motion, MouseButton, ResizeArgs,
};
use std::{collections::VecDeque, error::Error, time::Duration};
use window::{AdvancedWindow, BuildFromWindowSettings, Position, Size, Window, WindowSettings};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{
        ElementState,
        MouseScrollDelta,
        WindowEvent,
    },
    event_loop::{ActiveEventLoop, EventLoop},
    window::{CursorGrabMode, WindowId},
};

/// Settings for whether to ignore modifiers and use standard keyboard layouts instead.
///
/// This does not affect `piston::input::TextEvent`.
///
/// Piston uses the same key codes as in SDL2.
/// The problem is that without knowing the keyboard layout,
/// there is no coherent way of generating key codes.
///
/// This option choose different tradeoffs depending on need.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyboardIgnoreModifiers {
    /// Keep the key codes that are affected by modifiers.
    ///
    /// This is a good default for most applications.
    /// However, make sure to display understandable information to the user.
    ///
    /// If you experience user problems among gamers,
    /// then you might consider allowing other options in your game engine.
    /// Some gamers might be used to how stuff works in other traditional game engines
    /// and struggle understanding this configuration, depending on how you use keyboard layout.
    None,
    /// Assume the user's keyboard layout is standard English ABC.
    ///
    /// In some non-English speaking countries, this might be more user friendly for some gamers.
    ///
    /// This might sound counter-intuitive at first, so here is the reason:
    ///
    /// Gamers can customize their keyboard layout without needing to understand scan codes.
    /// When gamers want physically accuracy with good default options,
    /// they can simply use standard English ABC.
    ///
    /// In other cases, this option displays understandable information for game instructions.
    /// This information makes it easier for users to correct the problem themselves.
    ///
    /// Most gaming consoles use standard controllers.
    /// Typically, the only device that might be problematic for users is the keyboard.
    /// Instead of solving this problem in your game engine, let users do it in the OS.
    ///
    /// This option gives more control to users and is also better for user data privacy.
    /// Detecting keyboard layout is usually not needed.
    /// Instead, provide options for the user where they can modify the keys.
    /// If users want to switch layout in the middle of a game, they can do it through the OS.
    AbcKeyCode,
}

pub struct WinitWindow {
    /// The event loop of the window.
    ///
    /// This is optional because when pumping events using `ApplicationHandler`,
    /// the event loop can not be owned by `WinitWindow`.
    pub event_loop: Option<EventLoop<UserEvent>>,
    /// Sets keyboard layout.
    ///
    /// When set, the key codes are
    pub keyboard_ignore_modifiers: KeyboardIgnoreModifiers,
    /// The Winit window.
    ///
    /// This is optional because when creating the window,
    /// it is only accessible by `ActiveEventLoop::create_window`,
    /// which in turn requires `ApplicationHandler`.
    /// One call to `Window::pull_event` is needed to trigger
    /// Winit to call `ApplicationHandler::request_redraw`,
    /// which creates the window.
    pub window: Option<Arc<winit::window::Window>>,
    settings: WindowSettings,
    // The back-end does not remember the title.
    title: String,
    exit_on_esc: bool,
    should_close: bool,
    automatic_close: bool,
    last_cursor: LogicalPosition<f64>,
    cursor_accumulator: LogicalPosition<f64>,
    capture_cursor: bool,
    // Used to filter repeated key presses (does not affect text repeat).
    last_key_pressed: Option<input::Key>,
    // Stores list of events ready for processing.
    events: VecDeque<Event>,
}

/// Custom events for the winit event loop
#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    /// Do nothing, just spin the event loop
    WakeUp,
}

impl WinitWindow {
    pub fn new(settings: &WindowSettings) -> Self {
        let event_loop = EventLoop::with_user_event().build().unwrap();

        let mut w = WinitWindow {
            event_loop: Some(event_loop),
            keyboard_ignore_modifiers: KeyboardIgnoreModifiers::None,
            window: None,

            settings: settings.clone(),
            should_close: false,
            automatic_close: settings.get_automatic_close(),
            events: VecDeque::new(),
            last_cursor: LogicalPosition::new(0.0, 0.0),
            cursor_accumulator: LogicalPosition::new(0.0, 0.0),
            last_key_pressed: None,

            title: settings.get_title(),
            capture_cursor: false,
            exit_on_esc: settings.get_exit_on_esc(),
        };
        // Causes the window to be created through `ApplicationHandler::request_redraw`.
        if let Some(e) = w.poll_event() {w.events.push_front(e)}
        w
    }

    /// Gets a reference to the window.
    ///
    /// This is faster than [get_window], but borrows self.
    pub fn get_window_ref(&self) -> &winit::window::Window {
        self.window.as_ref().unwrap()
    }

    /// Returns a cloned smart pointer to the underlying Winit window.
    pub fn get_window(&self) -> Arc<winit::window::Window> {
        self.window.as_ref().unwrap().clone()
    }

    fn handle_event(
        &mut self,
        event: winit::event::WindowEvent,
        center: PhysicalPosition<f64>,
        unknown: &mut bool,
    ) -> Option<Input> {
        use winit::keyboard::{Key, NamedKey};

        match event {
            WindowEvent::KeyboardInput { event: ref ev, .. } => {
                if self.exit_on_esc {
                    if let Key::Named(NamedKey::Escape) = ev.logical_key {
                        self.set_should_close(true);
                        return None;
                    }
                }
                if let Some(s) = &ev.text {
                    let s = s.to_string();
                    let repeat = ev.repeat;
                    if !repeat {
                        if let Some(input) = map_window_event(
                            event,
                            self.get_window_ref().scale_factor(),
                            self.keyboard_ignore_modifiers,
                            unknown,
                            &mut self.last_key_pressed,
                        ) {
                            self.events.push_back(Event::Input(input, None));
                        }
                    }

                    return Some(Input::Text(s));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.capture_cursor {
                    let prev_last_cursor = self.last_cursor;
                    self.last_cursor =
                        position.to_logical(self.get_window_ref().scale_factor());

                    // Don't track distance if the position is at the center, this probably is
                    //  from cursor center lock, or irrelevant.
                    if position == center {
                        return None;
                    }

                    // Add the distance to the tracked cursor movement
                    self.cursor_accumulator.x += position.x - prev_last_cursor.x as f64;
                    self.cursor_accumulator.y += position.y - prev_last_cursor.y as f64;

                    return None;
                }
            }
            _ => {}
        }

        // Usual events are handled here and passed to user.
        map_window_event(
            event,
            self.get_window_ref().scale_factor(),
            self.keyboard_ignore_modifiers,
            unknown,
            &mut self.last_key_pressed,
        )
    }
}

impl Window for WinitWindow {
    fn set_should_close(&mut self, value: bool) {
        self.should_close = value;
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn size(&self) -> Size {
        let window = self.get_window_ref();
        let (w, h): (u32, u32) = window.inner_size().into();
        let hidpi = window.scale_factor();
        ((w as f64 / hidpi) as u32, (h as f64 / hidpi) as u32).into()
    }

    fn swap_buffers(&mut self) {
        // This window backend was made for use with a vulkan renderer that handles swapping by
        //  itself, if you need it here open up an issue. What we can use this for however is
        //  detecting the end of a frame, which we can use to gather up cursor_accumulator data.

        if self.capture_cursor {
            let center: (f64, f64) = self.get_window_ref().inner_size().into();
            let mut center: PhysicalPosition<f64> = center.into();
            center.x /= 2.;
            center.y /= 2.;

            // Center-lock the cursor if we're using capture_cursor
            self.get_window_ref().set_cursor_position(center).unwrap();

            // Create a relative input based on the distance from the center
            self.events.push_back(Event::Input(
                Input::Move(Motion::MouseRelative([
                    self.cursor_accumulator.x,
                    self.cursor_accumulator.y,
                ])),
                None,
            ));

            self.cursor_accumulator = LogicalPosition::new(0.0, 0.0);
        }
    }

    fn wait_event(&mut self) -> Event {
        use winit::platform::pump_events::EventLoopExtPumpEvents;
        use input::{IdleArgs, Loop};

        // Add all events we got to the event queue, since winit only allows us to get all pending
        //  events at once.
        if let Some(mut event_loop) = std::mem::replace(&mut self.event_loop, None) {
            let event_loop_proxy = event_loop.create_proxy();
            event_loop_proxy
                .send_event(UserEvent::WakeUp)
                .expect("Event loop is closed before property handling all events.");
            event_loop.pump_app_events(None, self);
            self.event_loop = Some(event_loop);
        }

        // Get the first event in the queue
        let event = self.events.pop_front();

        // Check if we got a close event, if we did we need to mark ourselves as should-close
        if let &Some(Event::Input(Input::Close(_), ..)) = &event {
            self.set_should_close(true);
        }

        event.unwrap_or(Event::Loop(Loop::Idle(IdleArgs {dt: 0.0})))
    }

    fn wait_event_timeout(&mut self, timeout: Duration) -> Option<Event> {
        use winit::platform::pump_events::EventLoopExtPumpEvents;

        // Add all events we got to the event queue, since winit only allows us to get all pending
        //  events at once.
        if let Some(mut event_loop) = std::mem::replace(&mut self.event_loop, None) {
            let event_loop_proxy = event_loop.create_proxy();
            event_loop_proxy
                .send_event(UserEvent::WakeUp)
                .expect("Event loop is closed before property handling all events.");
            event_loop.pump_app_events(Some(timeout), self);
            self.event_loop = Some(event_loop);
        }

        // Get the first event in the queue
        let event = self.events.pop_front();

        // Check if we got a close event, if we did we need to mark ourselves as should-close
        if let &Some(Event::Input(Input::Close(_), ..)) = &event {
            self.set_should_close(true);
        }

        event
    }

    fn poll_event(&mut self) -> Option<Event> {
        use winit::platform::pump_events::EventLoopExtPumpEvents;

        // Add all events we got to the event queue, since winit only allows us to get all pending
        //  events at once.
        if let Some(mut event_loop) = std::mem::replace(&mut self.event_loop, None) {
            let event_loop_proxy = event_loop.create_proxy();
            event_loop_proxy
                .send_event(UserEvent::WakeUp)
                .expect("Event loop is closed before property handling all events.");
            event_loop.pump_app_events(Some(Duration::ZERO), self);
            self.event_loop = Some(event_loop);
        }

        // Get the first event in the queue
        let event = self.events.pop_front();

        // Check if we got a close event, if we did we need to mark ourselves as should-close
        if let &Some(Event::Input(Input::Close(_), ..)) = &event {
            self.set_should_close(true);
        }

        event
    }

    fn draw_size(&self) -> Size {
        let size: (f64, f64) = self.get_window_ref().inner_size().into();
        size.into()
    }
}

impl ApplicationHandler<UserEvent> for WinitWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let settings = &self.settings;
        let window = event_loop.create_window(winit::window::Window::default_attributes()
            .with_inner_size(LogicalSize::<f64>::new(
                settings.get_size().width.into(),
                settings.get_size().height.into(),
            ))
            .with_title(settings.get_title())
        ).unwrap();
        self.window = Some(Arc::new(window));
    }

    fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            let window =  &self.get_window_ref();

            match event {
                WindowEvent::CloseRequested => {
                    if self.automatic_close {
                        self.should_close = true;
                        event_loop.exit();
                    }
                }
                WindowEvent::RedrawRequested => {
                    window.request_redraw();
                },
                event => {
                    let center: (f64, f64) = self.get_window_ref().inner_size().into();
                    let mut center: PhysicalPosition<f64> = center.into();
                    center.x /= 2.;
                    center.y /= 2.;

                    let mut unknown = false;
                    if let Some(ev) = self.handle_event(event, center, &mut unknown) {
                        if !unknown {
                            self.events.push_back(Event::Input(ev, None));
                        }
                    }
                }
            }
        }
}

impl AdvancedWindow for WinitWindow {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, value: String) {
        self.get_window_ref().set_title(&value);
        self.title = value;
    }

    fn get_exit_on_esc(&self) -> bool {
        self.exit_on_esc
    }

    fn set_exit_on_esc(&mut self, value: bool) {
        self.exit_on_esc = value
    }

    fn set_capture_cursor(&mut self, value: bool) {
        // If we're already doing this, just don't do anything
        if value == self.capture_cursor {
            return;
        }

        if value {
            self.cursor_accumulator = LogicalPosition::new(0.0, 0.0);
            let window = self.get_window_ref();
            window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
            window.set_cursor_visible(false);
            let mut center = window.inner_size().cast::<f64>();
            center.width /= 2.;
            center.height /= 2.;
            self.last_cursor = LogicalPosition::new(center.width, center.height);
        } else {
            let window = self.get_window_ref();
            window.set_cursor_grab(CursorGrabMode::None).unwrap();
            window.set_cursor_visible(true);
        }
        self.capture_cursor = value;
    }

    fn get_automatic_close(&self) -> bool {self.automatic_close}

    fn set_automatic_close(&mut self, value: bool) {self.automatic_close = value}

    fn show(&mut self) {
        self.get_window_ref().set_visible(true);
    }

    fn hide(&mut self) {
        self.get_window_ref().set_visible(false);
    }

    fn get_position(&self) -> Option<Position> {
        self.get_window_ref()
            .outer_position()
            .map(|p| Position { x: p.x, y: p.y })
            .ok()
    }

    fn set_position<P: Into<Position>>(&mut self, val: P) {
        let val = val.into();
        self.get_window_ref()
            .set_outer_position(LogicalPosition::new(val.x as f64, val.y as f64))
    }

    fn set_size<S: Into<Size>>(&mut self, size: S) {
        let size: Size = size.into();
        let w = self.get_window_ref();
        let hidpi = w.scale_factor();
        let _ = w.request_inner_size(LogicalSize::new(
            size.width as f64 * hidpi,
            size.height as f64 * hidpi,
        ));
    }
}

impl BuildFromWindowSettings for WinitWindow {
    fn build_from_window_settings(settings: &WindowSettings) -> Result<Self, Box<dyn Error>> {
        Ok(Self::new(settings))
    }
}

fn map_key(input: &winit::event::KeyEvent, kim: KeyboardIgnoreModifiers) -> Key {
    use winit::keyboard::NamedKey::*;
    use winit::keyboard::Key::*;
    use KeyboardIgnoreModifiers as KIM;

    // TODO: Complete the lookup match
    match input.logical_key {
        Character(ref ch) => match ch.as_str() {
            "0" | ")" if kim == KIM::AbcKeyCode => Key::D0,
            "0" => Key::D0,
            ")" => Key::RightParen,
            "1" | "!" if kim == KIM::AbcKeyCode => Key::D1,
            "1" => Key::D1,
            "!" => Key::NumPadExclam,
            "2" | "@" if kim == KIM::AbcKeyCode => Key::D2,
            "2" => Key::D2,
            "@" => Key::At,
            "3" | "#" if kim == KIM::AbcKeyCode => Key::D3,
            "3" => Key::D3,
            "#" => Key::Hash,
            "4" | "$" if kim == KIM::AbcKeyCode => Key::D4,
            "4" => Key::D4,
            "$" => Key::Dollar,
            "5" | "%" if kim == KIM::AbcKeyCode => Key::D5,
            "5" => Key::D5,
            "%" => Key::Percent,
            "6" | "^" if kim == KIM::AbcKeyCode => Key::D6,
            "6" => Key::D6,
            "^" => Key::Caret,
            "7" | "&" if kim == KIM::AbcKeyCode => Key::D7,
            "7" => Key::D7,
            "&" => Key::Ampersand,
            "8" | "*" if kim == KIM::AbcKeyCode => Key::D8,
            "8" => Key::D8,
            "*" => Key::Asterisk,
            "9" | "(" if kim == KIM::AbcKeyCode => Key::D9,
            "9" => Key::D9,
            "(" => Key::LeftParen,
            "a" | "A" => Key::A,
            "b" | "B" => Key::B,
            "c" | "C" => Key::C,
            "d" | "D" => Key::D,
            "e" | "E" => Key::E,
            "f" | "F" => Key::F,
            "g" | "G" => Key::G,
            "h" | "H" => Key::H,
            "i" | "I" => Key::I,
            "j" | "J" => Key::J,
            "k" | "K" => Key::K,
            "l" | "L" => Key::L,
            "m" | "M" => Key::M,
            "n" | "N" => Key::N,
            "o" | "O" => Key::O,
            "p" | "P" => Key::P,
            "q" | "Q" => Key::Q,
            "r" | "R" => Key::R,
            "s" | "S" => Key::S,
            "t" | "T" => Key::T,
            "u" | "U" => Key::U,
            "v" | "V" => Key::V,
            "w" | "W" => Key::W,
            "x" | "X" => Key::X,
            "y" | "Y" => Key::Y,
            "z" | "Z" => Key::Z,
            "'" | "\"" if kim == KIM::AbcKeyCode => Key::Quote,
            "'" => Key::Quote,
            "\"" => Key::Quotedbl,
            ";" | ":" if kim == KIM::AbcKeyCode => Key::Semicolon,
            ";" => Key::Semicolon,
            ":" => Key::Colon,
            "[" | "{" if kim == KIM::AbcKeyCode => Key::LeftBracket,
            "[" => Key::LeftBracket,
            "{" => Key::NumPadLeftBrace,
            "]" | "}" if kim == KIM::AbcKeyCode => Key::RightBracket,
            "]" => Key::RightBracket,
            "}" => Key::NumPadRightBrace,
            "\\" | "|" if kim == KIM::AbcKeyCode => Key::Backslash,
            "\\" => Key::Backslash,
            "|" => Key::NumPadVerticalBar,
            "," | "<" if kim == KIM::AbcKeyCode => Key::Comma,
            "," => Key::Comma,
            "<" => Key::Less,
            "." | ">" if kim == KIM::AbcKeyCode => Key::Period,
            "." => Key::Period,
            ">" => Key::Greater,
            "/" | "?" if kim == KIM::AbcKeyCode => Key::Slash,
            "/" => Key::Slash,
            "?" => Key::Question,
            "`" | "~" if kim == KIM::AbcKeyCode => Key::Backquote,
            "`" => Key::Backquote,
            // Piston v1.0 does not support `~` using modifier.
            // Use `KeyboardIgnoreModifiers::AbcKeyCode` on window to fix this issue.
            // It will be mapped to `Key::Backquote`.
            "~" => Key::Unknown,
            _ => Key::Unknown,
        }
        Named(Escape) => Key::Escape,
        Named(F1) => Key::F1,
        Named(F2) => Key::F2,
        Named(F3) => Key::F3,
        Named(F4) => Key::F4,
        Named(F5) => Key::F5,
        Named(F6) => Key::F6,
        Named(F7) => Key::F7,
        Named(F8) => Key::F8,
        Named(F9) => Key::F9,
        Named(F10) => Key::F10,
        Named(F11) => Key::F11,
        Named(F12) => Key::F12,
        Named(F13) => Key::F13,
        Named(F14) => Key::F14,
        Named(F15) => Key::F15,

        Named(Delete) => Key::Delete,

        Named(ArrowLeft) => Key::Left,
        Named(ArrowUp) => Key::Up,
        Named(ArrowRight) => Key::Right,
        Named(ArrowDown) => Key::Down,

        Named(Backspace) => Key::Backspace,
        Named(Enter) => Key::Return,
        Named(Space) => Key::Space,

        Named(Alt) => Key::LAlt,
        Named(AltGraph) => Key::RAlt,
        Named(Control) => Key::LCtrl,
        Named(Super) => Key::Menu,
        Named(Shift) => Key::LShift,

        Named(Tab) => Key::Tab,
        _ => Key::Unknown,
    }
}

fn map_keyboard_input(
    input: &winit::event::KeyEvent,
    kim: KeyboardIgnoreModifiers,
    unknown: &mut bool,
    last_key_pressed: &mut Option<Key>,
) -> Option<Input> {
    let key = map_key(input, kim);

    let state = if input.state == ElementState::Pressed {
        // Filter repeated key presses (does not affect text repeat when holding keys).
        if let Some(last_key) = &*last_key_pressed {
            if last_key == &key {
                *unknown = true;
                return None;
            }
        }
        *last_key_pressed = Some(key);

        ButtonState::Press
    } else {
        if let Some(last_key) = &*last_key_pressed {
            if last_key == &key {
                *last_key_pressed = None;
            }
        }
        ButtonState::Release
    };

    Some(Input::Button(ButtonArgs {
        state: state,
        button: Button::Keyboard(key),
        scancode: if let winit::keyboard::PhysicalKey::Code(code) = input.physical_key {
                Some(code as i32)
            } else {None},
    }))
}

/// Maps Winit's mouse button to Piston's mouse button.
pub fn map_mouse(mouse_button: winit::event::MouseButton) -> MouseButton {
    use winit::event::MouseButton as M;

    match mouse_button {
        M::Left => MouseButton::Left,
        M::Right => MouseButton::Right,
        M::Middle => MouseButton::Middle,
        M::Other(0) => MouseButton::X1,
        M::Other(1) => MouseButton::X2,
        M::Other(2) => MouseButton::Button6,
        M::Other(3) => MouseButton::Button7,
        M::Other(4) => MouseButton::Button8,
        _ => MouseButton::Unknown
    }
}

/// Converts a winit's [`WindowEvent`] into a piston's [`Input`].
///
/// For some events that will not be passed to the user, returns `None`.
fn map_window_event(
    window_event: WindowEvent,
    scale_factor: f64,
    kim: KeyboardIgnoreModifiers,
    unknown: &mut bool,
    last_key_pressed: &mut Option<Key>,
) -> Option<Input> {
    use input::FileDrag;

    match window_event {
        WindowEvent::DroppedFile(path) =>
            Some(Input::FileDrag(FileDrag::Drop(path))),
        WindowEvent::HoveredFile(path) =>
            Some(Input::FileDrag(FileDrag::Hover(path))),
        WindowEvent::HoveredFileCancelled =>
            Some(Input::FileDrag(FileDrag::Cancel)),
        WindowEvent::Resized(size) => Some(Input::Resize(ResizeArgs {
            window_size: [size.width as f64, size.height as f64],
            draw_size: Size {
                width: size.width as f64,
                height: size.height as f64,
            }
            .into(),
        })),
        // TODO: Implement this
        WindowEvent::Moved(_) => None,
        WindowEvent::CloseRequested => Some(Input::Close(CloseArgs)),
        WindowEvent::Destroyed => Some(Input::Close(CloseArgs)),
        WindowEvent::Focused(focused) => Some(Input::Focus(focused)),
        WindowEvent::KeyboardInput { ref event, .. } => {
            map_keyboard_input(event, kim, unknown, last_key_pressed)
        }
        // TODO: Implement this
        WindowEvent::ModifiersChanged(_) => None,
        WindowEvent::CursorMoved { position, .. } => {
            let position = position.to_logical(scale_factor);
            Some(Input::Move(Motion::MouseCursor([position.x, position.y])))
        }
        WindowEvent::CursorEntered { .. } => Some(Input::Cursor(true)),
        WindowEvent::CursorLeft { .. } => Some(Input::Cursor(false)),
        WindowEvent::MouseWheel { delta, .. } => match delta {
            MouseScrollDelta::PixelDelta(position) => {
                let position = position.to_logical(scale_factor);
                Some(Input::Move(Motion::MouseScroll([position.x, position.y])))
            }
            MouseScrollDelta::LineDelta(x, y) =>
                Some(Input::Move(Motion::MouseScroll([x as f64, y as f64]))),
        },
        WindowEvent::MouseInput { state, button, .. } => {
            let button = map_mouse(button);
            let state = match state {
                ElementState::Pressed => ButtonState::Press,
                ElementState::Released => ButtonState::Release,
            };

            Some(Input::Button(ButtonArgs {
                state,
                button: Button::Mouse(button),
                scancode: None,
            }))
        }
        // TODO: Implement this
        WindowEvent::TouchpadPressure { .. } |
        WindowEvent::PinchGesture { .. } |
        WindowEvent::RotationGesture { .. } |
        WindowEvent::PanGesture { .. } |
        WindowEvent::DoubleTapGesture { .. } => None,
        // TODO: Implement this
        WindowEvent::AxisMotion { .. } => None,
        // TODO: Implement this
        WindowEvent::Touch(_) => None,
        // TODO: Implement this
        WindowEvent::ScaleFactorChanged { .. } => None,
        // TODO: Implement this
        WindowEvent::ActivationTokenDone { .. } => None,
        // TODO: Implement this
        WindowEvent::ThemeChanged(_) => None,
        // TODO: Implement this
        WindowEvent::Ime(_) => None,
        // TODO: Implement this
        WindowEvent::Occluded(_) => None,
        // TODO: Implement this
        WindowEvent::RedrawRequested { .. } => None,
    }
}

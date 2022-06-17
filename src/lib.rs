//! A [winit](https://crates.io/crates/winit) window back-end for the Piston game engine.

extern crate input;
#[cfg(feature = "use-vulkano")]
extern crate vulkano;
#[cfg(feature = "use-vulkano")]
extern crate vulkano_win;
extern crate window;
extern crate winit;

#[cfg(feature = "use-vulkano")]
mod vulkano_window;

#[cfg(feature = "use-vulkano")]
pub use vulkano_window::{required_extensions, VulkanoWindow};

use input::{
    Button, ButtonArgs, ButtonState, CloseArgs, Event, Input, Key, Motion, MouseButton, ResizeArgs,
};
use std::{collections::VecDeque, error::Error, time::Duration};
use window::{AdvancedWindow, BuildFromWindowSettings, Position, Size, Window, WindowSettings};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{
        ElementState, KeyboardInput, MouseButton as WinitMouseButton, MouseScrollDelta,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

pub struct WinitWindow {
    // TODO: These public fields should be changed to accessors
    pub event_loop: EventLoop<UserEvent>,

    window: winit::window::Window,

    should_close: bool,
    queued_events: VecDeque<Event>,
    last_cursor: LogicalPosition<f64>,
    cursor_accumulator: LogicalPosition<f64>,

    title: String,
    capture_cursor: bool,
    exit_on_esc: bool,
}

/// Custom events for the winit event loop
#[derive(Debug, PartialEq, Eq)]
pub enum UserEvent {
    /// Do nothing, just spin the event loop
    WakeUp,
}

impl WinitWindow {
    pub fn new(settings: &WindowSettings) -> Self {
        let event_loop = EventLoop::with_user_event();
        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize::<f64>::new(
                settings.get_size().width.into(),
                settings.get_size().height.into(),
            ))
            .with_title(settings.get_title())
            .build(&event_loop)
            .unwrap();

        WinitWindow {
            window,
            event_loop,

            should_close: false,
            queued_events: VecDeque::new(),
            last_cursor: LogicalPosition::new(0.0, 0.0),
            cursor_accumulator: LogicalPosition::new(0.0, 0.0),

            title: settings.get_title(),
            capture_cursor: false,
            exit_on_esc: settings.get_exit_on_esc(),
        }
    }

    pub fn get_window(&self) -> &winit::window::Window {
        &self.window
    }

    fn handle_event<T>(&mut self, event: winit::event::Event<T>, center: PhysicalPosition<f64>) {
        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                // Special event handling.
                // Some events are not exposed to user and handled internally.
                match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        if self.exit_on_esc {
                            if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                                self.set_should_close(true);
                                return;
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if self.capture_cursor {
                            let prev_last_cursor = self.last_cursor;
                            self.last_cursor =
                                position.to_logical(self.get_window().scale_factor());

                            // Don't track distance if the position is at the center, this probably is
                            //  from cursor center lock, or irrelevant.
                            if position == center {
                                return;
                            }

                            // Add the distance to the tracked cursor movement
                            self.cursor_accumulator.x += position.x - prev_last_cursor.x as f64;
                            self.cursor_accumulator.y += position.y - prev_last_cursor.y as f64;

                            return;
                        }
                    }
                    _ => {}
                }

                // Usual events are handled here and passed to user.
                if let Some(ev) = map_window_event(event) {
                    self.queued_events.push_back(ev);
                }
            }
            _ => (),
        }
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
        let (w, h): (u32, u32) = self.get_window().inner_size().into();
        let hidpi = self.get_window().scale_factor();
        ((w as f64 / hidpi) as u32, (h as f64 / hidpi) as u32).into()
    }

    fn swap_buffers(&mut self) {
        // This window backend was made for use with a vulkan renderer that handles swapping by
        //  itself, if you need it here open up an issue. What we can use this for however is
        //  detecting the end of a frame, which we can use to gather up cursor_accumulator data.

        if self.capture_cursor {
            let center: (f64, f64) = self.get_window().inner_size().into();
            let mut center: PhysicalPosition<f64> = center.into();
            center.x /= 2.;
            center.y /= 2.;

            // Center-lock the cursor if we're using capture_cursor
            self.get_window().set_cursor_position(center).unwrap();

            // Create a relative input based on the distance from the center
            self.queued_events.push_back(Event::Input(
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
        // TODO: Implement this
        unimplemented!()
    }

    fn wait_event_timeout(&mut self, _timeout: Duration) -> Option<Event> {
        // TODO: Implement this
        unimplemented!()
    }

    fn poll_event(&mut self) -> Option<Event> {
        let center: (f64, f64) = self.get_window().inner_size().into();
        let mut center: PhysicalPosition<f64> = center.into();
        center.x /= 2.;
        center.y /= 2.;

        // Add all events we got to the event queue, since winit only allows us to get all pending
        //  events at once.
        {
            let mut events: Vec<winit::event::Event<UserEvent>> = Vec::new();
            let event_loop_proxy = self.event_loop.create_proxy();
            event_loop_proxy
                .send_event(UserEvent::WakeUp)
                .expect("Event loop is closed before property handling all events.");

            self.event_loop.run_return(|event, _, control_flow| {
                if let Some(e) = event.to_static() {
                    if e == winit::event::Event::UserEvent(UserEvent::WakeUp) {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    events.push(e);
                }
            });
            for event in events.into_iter() {
                self.handle_event(event, center)
            }
        }

        // Get the first event in the queue
        let event = self.queued_events.pop_front();

        // Check if we got a close event, if we did we need to mark ourselves as should-close
        if let &Some(Event::Input(Input::Close(_), ..)) = &event {
            self.set_should_close(true);
        }

        event
    }

    fn draw_size(&self) -> Size {
        let size: (f64, f64) = self.get_window().inner_size().into();
        size.into()
    }
}

impl AdvancedWindow for WinitWindow {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, value: String) {
        self.get_window().set_title(&value);
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

        let window = self.get_window();
        if value {
            window.set_cursor_grab(true).unwrap();
            window.set_cursor_visible(false);
            self.cursor_accumulator = LogicalPosition::new(0.0, 0.0);
            let mut center = self.get_window().inner_size().cast::<f64>();
            center.width /= 2.;
            center.height /= 2.;
            self.last_cursor = LogicalPosition::new(center.width, center.height);
        } else {
            window.set_cursor_grab(false).unwrap();
            window.set_cursor_visible(true);
        }
        self.capture_cursor = value;
    }

    fn get_automatic_close(&self) -> bool {
        false
    }

    fn set_automatic_close(&mut self, _value: bool) {
        // TODO: Implement this
    }

    fn show(&mut self) {
        self.get_window().set_visible(true);
    }

    fn hide(&mut self) {
        self.get_window().set_visible(false);
    }

    fn get_position(&self) -> Option<Position> {
        self.get_window()
            .outer_position()
            .map(|p| Position { x: p.x, y: p.y })
            .ok()
    }

    fn set_position<P: Into<Position>>(&mut self, val: P) {
        let val = val.into();
        self.get_window()
            .set_outer_position(LogicalPosition::new(val.x as f64, val.y as f64))
    }

    fn set_size<S: Into<Size>>(&mut self, size: S) {
        let size: Size = size.into();
        let hidpi = self.get_window().scale_factor();
        self.get_window().set_inner_size(LogicalSize::new(
            size.width as f64 * hidpi,
            size.height as f64 * hidpi,
        ));
    }
}

#[cfg(not(feature = "use-vulkano"))]
impl BuildFromWindowSettings for WinitWindow {
    fn build_from_window_settings(settings: &WindowSettings) -> Result<Self, Box<dyn Error>> {
        Ok(Self::new(settings))
    }
}

fn map_key(input: &KeyboardInput) -> Key {
    use winit::event::VirtualKeyCode::*;
    // TODO: Complete the lookup match
    if let Some(vk) = input.virtual_keycode {
        match vk {
            Key1 => Key::D1,
            Key2 => Key::D2,
            Key3 => Key::D3,
            Key4 => Key::D4,
            Key5 => Key::D5,
            Key6 => Key::D6,
            Key7 => Key::D7,
            Key8 => Key::D8,
            Key9 => Key::D9,
            Key0 => Key::D0,
            A => Key::A,
            B => Key::B,
            C => Key::C,
            D => Key::D,
            E => Key::E,
            F => Key::F,
            G => Key::G,
            H => Key::H,
            I => Key::I,
            J => Key::J,
            K => Key::K,
            L => Key::L,
            M => Key::M,
            N => Key::N,
            O => Key::O,
            P => Key::P,
            Q => Key::Q,
            R => Key::R,
            S => Key::S,
            T => Key::T,
            U => Key::U,
            V => Key::V,
            W => Key::W,
            X => Key::X,
            Y => Key::Y,
            Z => Key::Z,
            Escape => Key::Escape,
            F1 => Key::F1,
            F2 => Key::F2,
            F3 => Key::F3,
            F4 => Key::F4,
            F5 => Key::F5,
            F6 => Key::F6,
            F7 => Key::F7,
            F8 => Key::F8,
            F9 => Key::F9,
            F10 => Key::F10,
            F11 => Key::F11,
            F12 => Key::F12,
            F13 => Key::F13,
            F14 => Key::F14,
            F15 => Key::F15,

            Delete => Key::Delete,

            Left => Key::Left,
            Up => Key::Up,
            Right => Key::Right,
            Down => Key::Down,

            Back => Key::Backspace,
            Return => Key::Return,
            Space => Key::Space,

            LAlt => Key::LAlt,
            LControl => Key::LCtrl,
            LWin => Key::Menu,
            LShift => Key::LShift,

            RAlt => Key::LAlt,
            RControl => Key::RCtrl,
            RWin => Key::Menu,
            RShift => Key::RShift,

            Tab => Key::Tab,
            _ => Key::Unknown,
        }
    } else {
        Key::Unknown
    }
}

fn map_keyboard_input(input: &KeyboardInput) -> Event {
    let key = map_key(input);

    let state = if input.state == ElementState::Pressed {
        ButtonState::Press
    } else {
        ButtonState::Release
    };

    Event::Input(
        Input::Button(ButtonArgs {
            state: state,
            button: Button::Keyboard(key),
            scancode: Some(input.scancode as i32),
        }),
        None,
    )
}

fn map_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Other(4) => MouseButton::X1,
        WinitMouseButton::Other(5) => MouseButton::X2,
        WinitMouseButton::Other(6) => MouseButton::Button6,
        WinitMouseButton::Other(7) => MouseButton::Button7,
        WinitMouseButton::Other(8) => MouseButton::Button8,
        _ => MouseButton::Unknown,
    }
}

/// Converts a winit's [`WindowEvent`] into a piston's [`Event`].
///
/// For some events that will not be passed to the user, returns `None`.
fn map_window_event(window_evnet: WindowEvent) -> Option<Event> {
    match window_evnet {
        // TODO: This event needs to be added to pistoncore-input, see issue
        //  PistonDevelopers/piston#1117
        //WindowEvent::DroppedFile(path) => {
        //    Input::Custom(EventId("DroppedFile"), Arc::new(path))
        //},
        WindowEvent::Resized(size) => Some(Event::Input(
            Input::Resize(ResizeArgs {
                window_size: [size.width as f64, size.height as f64],
                draw_size: Size {
                    width: size.width as f64,
                    height: size.height as f64,
                }
                .into(),
            }),
            None,
        )),
        // TODO: Implement this
        WindowEvent::Moved(_) => None,
        WindowEvent::CloseRequested => Some(Event::Input(Input::Close(CloseArgs), None)),
        // TODO: Implement this
        WindowEvent::Destroyed => None,
        // TODO: Implement this
        WindowEvent::DroppedFile(_) => None,
        // TODO: Implement this
        WindowEvent::HoveredFile(_) => None,
        // TODO: Implement this
        WindowEvent::HoveredFileCancelled => None,
        WindowEvent::ReceivedCharacter(c) => match c {
            // Ignore control characters
            '\u{7f}' | // Delete
            '\u{1b}' | // Escape
            '\u{8}'  | // Backspace
            '\r' | '\n' | '\t' => None,
            _ => Some(Event::Input(Input::Text(c.to_string()), None)),
        },
        WindowEvent::Focused(focused) => Some(Event::Input(Input::Focus(focused), None)),
        WindowEvent::KeyboardInput { input, .. } => Some(map_keyboard_input(&input)),
        // TODO: Implement this
        WindowEvent::ModifiersChanged(_) => None,
        WindowEvent::CursorMoved { position, .. } => Some(Event::Input(
            Input::Move(Motion::MouseCursor([position.x, position.y])),
            None,
        )),
        WindowEvent::CursorEntered { .. } => Some(Event::Input(Input::Cursor(true), None)),
        WindowEvent::CursorLeft { .. } => Some(Event::Input(Input::Cursor(false), None)),
        WindowEvent::MouseWheel { delta, .. } => Some(match delta {
            MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => {
                Event::Input(Input::Move(Motion::MouseScroll([x as f64, y as f64])), None)
            }
            MouseScrollDelta::LineDelta(x, y) => {
                Event::Input(Input::Move(Motion::MouseScroll([x as f64, y as f64])), None)
            }
        }),
        WindowEvent::MouseInput { state, button, .. } => Some({
            let button = map_mouse_button(button);
            let state = match state {
                ElementState::Pressed => ButtonState::Press,
                ElementState::Released => ButtonState::Release,
            };

            Event::Input(
                Input::Button(ButtonArgs {
                    state,
                    button: Button::Mouse(button),
                    scancode: None,
                }),
                None,
            )
        }),
        // TODO: Implement this
        WindowEvent::TouchpadPressure { .. } => None,
        // TODO: Implement this
        WindowEvent::AxisMotion { .. } => None,
        // TODO: Implement this
        WindowEvent::Touch(_) => None,
        // TODO: Implement this
        WindowEvent::ScaleFactorChanged { .. } => None,
        // TODO: Implement this
        WindowEvent::ThemeChanged(_) => None,
    }
}

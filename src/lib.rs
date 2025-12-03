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

use std::sync::Arc;

#[cfg(feature = "use-vulkano")]
pub use vulkano_window::{required_extensions, VulkanoWindow};

use input::{
    Button, ButtonArgs, ButtonState, CloseArgs, Event, Input, Key, Motion, MouseButton, ResizeArgs,
};
use std::{collections::VecDeque, error::Error, time::Duration};
use window::{AdvancedWindow, BuildFromWindowSettings, Position, Size, Window, WindowSettings};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{
        ElementState, MouseButton as WinitMouseButton, MouseScrollDelta,
        WindowEvent,
    },
    event_loop::{ActiveEventLoop, EventLoop},
    window::{CursorGrabMode, WindowId},
};

pub struct WinitWindow {
    /// The event loop of the window.
    ///
    /// This is optional because when pumping events using `ApplicationHandler`,
    /// the event loop can not be owned by `WinitWindow`.
    pub event_loop: Option<EventLoop<UserEvent>>,
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
        let event_loop = EventLoop::with_user_event().build().unwrap();

        let mut w = WinitWindow {
            window: None,
            event_loop: Some(event_loop),

            settings: settings.clone(),
            should_close: false,
            queued_events: VecDeque::new(),
            last_cursor: LogicalPosition::new(0.0, 0.0),
            cursor_accumulator: LogicalPosition::new(0.0, 0.0),

            title: settings.get_title(),
            capture_cursor: false,
            exit_on_esc: settings.get_exit_on_esc(),
        };
        // Causes the window to be created through `ApplicationHandler::request_redraw`.
        if let Some(e) = w.poll_event() {w.queued_events.push_front(e)}
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

    fn handle_event(&mut self, event: winit::event::WindowEvent, center: PhysicalPosition<f64>) {
        use winit::keyboard::{Key, NamedKey};

        match event {
            WindowEvent::KeyboardInput { ref event, .. } => {
                if self.exit_on_esc {
                    if let Key::Named(NamedKey::Escape) = event.logical_key {
                        self.set_should_close(true);
                        return;
                    }
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
        let event = self.queued_events.pop_front();

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
        let event = self.queued_events.pop_front();

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
        let event = self.queued_events.pop_front();

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
                    self.should_close = true;
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    window.request_redraw();
                },
                event => {
                    let center: (f64, f64) = self.get_window_ref().inner_size().into();
                    let mut center: PhysicalPosition<f64> = center.into();
                    center.x /= 2.;
                    center.y /= 2.;

                    self.handle_event(event, center)
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

    fn get_automatic_close(&self) -> bool {
        false
    }

    fn set_automatic_close(&mut self, _value: bool) {
        // TODO: Implement this
    }

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

#[cfg(not(feature = "use-vulkano"))]
impl BuildFromWindowSettings for WinitWindow {
    fn build_from_window_settings(settings: &WindowSettings) -> Result<Self, Box<dyn Error>> {
        Ok(Self::new(settings))
    }
}

fn map_key(input: &winit::event::KeyEvent) -> Key {
    use winit::keyboard::NamedKey::*;
    use winit::keyboard::Key::*;

    // TODO: Complete the lookup match
    match input.logical_key {
        Character(ref ch) => match ch.as_str() {
            "0" => Key::D0,
            "1" => Key::D1,
            "2" => Key::D2,
            "3" => Key::D3,
            "4" => Key::D4,
            "5" => Key::D5,
            "6" => Key::D6,
            "7" => Key::D7,
            "8" => Key::D8,
            "9" => Key::D9,
            "a" => Key::A,
            "b" => Key::B,
            "c" => Key::C,
            "d" => Key::D,
            "e" => Key::E,
            "f" => Key::F,
            "g" => Key::G,
            "h" => Key::H,
            "i" => Key::I,
            "j" => Key::J,
            "k" => Key::K,
            "l" => Key::L,
            "m" => Key::M,
            "n" => Key::N,
            "o" => Key::O,
            "p" => Key::P,
            "q" => Key::Q,
            "r" => Key::R,
            "s" => Key::S,
            "t" => Key::T,
            "u" => Key::U,
            "v" => Key::V,
            "w" => Key::W,
            "x" => Key::X,
            "y" => Key::Y,
            "z" => Key::Z,
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

fn map_keyboard_input(input: &winit::event::KeyEvent) -> Event {
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
            scancode: if let winit::keyboard::PhysicalKey::Code(code) = input.physical_key {
                    Some(code as i32)
                } else {None},
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
fn map_window_event(window_event: WindowEvent) -> Option<Event> {
    use input::FileDrag;

    match window_event {
        WindowEvent::DroppedFile(path) =>
            Some(Event::Input(Input::FileDrag(FileDrag::Drop(path)), None)),
        WindowEvent::HoveredFile(path) =>
            Some(Event::Input(Input::FileDrag(FileDrag::Hover(path)), None)),
        WindowEvent::HoveredFileCancelled =>
            Some(Event::Input(Input::FileDrag(FileDrag::Cancel), None)),
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
        WindowEvent::Destroyed => Some(Event::Input(Input::Close(CloseArgs), None)),
        WindowEvent::Focused(focused) => Some(Event::Input(Input::Focus(focused), None)),
        WindowEvent::KeyboardInput { ref event, .. } => {
            Some(map_keyboard_input(event))
        }
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

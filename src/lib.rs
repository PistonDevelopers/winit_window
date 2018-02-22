extern crate winit;
extern crate vulkano;
extern crate vulkano_win;
extern crate input;
extern crate window;

use std::time::{Duration};
use std::sync::{Arc};
use std::collections::{VecDeque};

use vulkano::swapchain::{Surface};
use vulkano::instance::{Instance, InstanceExtensions};
use vulkano_win::{VkSurfaceBuild, Window as VulkanoWinWindow};
use winit::{EventsLoop, WindowBuilder, Event as WinitEvent, WindowEvent, ElementState, MouseButton as WinitMouseButton, KeyboardInput, MouseScrollDelta, CursorState};
use input::{Input, CloseArgs, Motion, Button, MouseButton, Key, ButtonState, ButtonArgs};
use window::{Window, Size, WindowSettings, Position, AdvancedWindow};

pub fn required_extensions() -> InstanceExtensions {
    vulkano_win::required_extensions()
}

pub struct WinitWindow {
    // TODO: These public fields should be changed to accessors
    pub window: VulkanoWinWindow,
    pub events_loop: EventsLoop,
    pub surface: Arc<Surface>,

    should_close: bool,
    queued_events: VecDeque<Input>,
    last_cursor: (f64, f64),
    cursor_accumulator: (f64, f64),

    title: String,
    capture_cursor: bool,
}

impl WinitWindow {
    pub fn new_vulkano(instance: Arc<Instance>, settings: &WindowSettings) -> Self {
        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new()
            .with_dimensions(settings.get_size().width, settings.get_size().height)
            .with_title(settings.get_title())
            .build_vk_surface(&events_loop, instance)
            .unwrap();

        let surface = window.surface().clone();

        WinitWindow {
            window,
            events_loop,
            surface,

            should_close: false,
            queued_events: VecDeque::new(),
            last_cursor: (0.0, 0.0),
            cursor_accumulator: (0.0, 0.0),

            title: settings.get_title(),
            capture_cursor: false,
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
        let (w, h) = self.window.window().get_inner_size().unwrap_or((1, 1));
        let hidpi = self.window.window().hidpi_factor();
        ((w as f32 / hidpi) as u32, (h as f32 / hidpi) as u32).into()
    }

    fn swap_buffers(&mut self) {
        // This window backend was made for use with a vulkan renderer that handles swapping by
        //  itself, if you need it here open up an issue. What we can use this for however is
        //  detecting the end of a frame, which we can use to gather up cursor_accumulator data.

        if self.capture_cursor {
            let mut center = self.window.window().get_inner_size().unwrap_or((2, 2));
            center.0 /= 2;
            center.1 /= 2;

            // Center-lock the cursor if we're using capture_cursor
            self.window.window().set_cursor_position(
                center.0 as i32, center.1 as i32
            ).unwrap();

            // Create a relative input based on the distance from the center
            self.queued_events.push_back(Input::Move(Motion::MouseRelative(
                self.cursor_accumulator.0,
                self.cursor_accumulator.1,
            )));

            self.cursor_accumulator = (0.0, 0.0);
        }
    }

    fn wait_event(&mut self) -> Input {
        // TODO: Implement this
        unimplemented!()
    }

    fn wait_event_timeout(&mut self, _timeout: Duration) -> Option<Input> {
        // TODO: Implement this
        unimplemented!()
    }

    fn poll_event(&mut self) -> Option<Input> {
        let mut center = self.window.window().get_inner_size().unwrap_or((2, 2));
        center.0 /= 2;
        center.1 /= 2;

        // Add all events we got to the event queue, since winit only allows us to get all pending
        //  events at once.
        {
            let queued_events = &mut self.queued_events;
            let capture_cursor = self.capture_cursor;
            let last_cursor = &mut self.last_cursor;
            let cursor_accumulator = &mut self.cursor_accumulator;
            self.events_loop.poll_events(|event| {
                push_events_for(
                    event, queued_events, capture_cursor, center,
                    last_cursor, cursor_accumulator,
                )
            });
        }

        // Get the first event in the queue
        let event = self.queued_events.pop_front();

        // Check if we got a close event, if we did we need to mark ourselves as should-close
        if let &Some(Input::Close(_)) = &event {
            self.set_should_close(true);
        }

        event
    }

    fn draw_size(&self) -> Size {
        self.window.window().get_inner_size()
            .unwrap_or((1, 1)).into()
    }
}

impl AdvancedWindow for WinitWindow {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, value: String) {
        self.window.window().set_title(&value);
        self.title = value;
    }

    fn get_exit_on_esc(&self) -> bool {
        false
    }

    fn set_exit_on_esc(&mut self, _value: bool) {
        // TODO: Implement this
    }

    fn set_capture_cursor(&mut self, value: bool) {
        // If we're already doing this, just don't do anything
        if value == self.capture_cursor {
            return;
        }

        if value {
            self.window.window().set_cursor_state(CursorState::Grab).unwrap();
            self.cursor_accumulator = (0.0, 0.0);
            let mut center = self.window.window().get_inner_size().unwrap_or((2, 2));
            center.0 /= 2;
            center.1 /= 2;
            self.last_cursor = (center.0 as f64, center.1 as f64);
        } else {
            self.window.window().set_cursor_state(CursorState::Normal).unwrap();
        }
        self.capture_cursor = value;
    }

    fn show(&mut self) {
        self.window.window().show();
    }

    fn hide(&mut self) {
        self.window.window().hide();
    }

    fn get_position(&self) -> Option<Position> {
        self.window.window().get_position().map(|p| Position { x: p.0, y: p.1 })
    }

    fn set_position<P: Into<Position>>(&mut self, val: P) {
        let val = val.into();
        self.window.window().set_position(val.x, val.y)
    }

    fn set_size<S: Into<Size>>(&mut self, size: S) {
        let size: Size = size.into();
        let hidpi = self.window.window().hidpi_factor();
        self.window.window().set_inner_size(
            (size.width as f32 * hidpi) as u32,
            (size.height as f32 * hidpi) as u32
        );
    }
}

fn push_events_for(
    event: WinitEvent, queue: &mut VecDeque<Input>,
    capture_cursor: bool, center: (u32, u32),
    last_cursor: &mut (f64, f64), cursor_accumulator: &mut (f64, f64),
) {
    match event {
        WinitEvent::WindowEvent { event: ev, .. } => {
            match ev {
                WindowEvent::Resized(w, h) => queue.push_back(Input::Resize(w, h)),
                WindowEvent::Closed => queue.push_back(Input::Close(CloseArgs)),
                // TODO: This event needs to be added to pistoncore-input, see issue
                //  PistonDevelopers/piston#1117
                //WindowEvent::DroppedFile(path) => {
                //    Input::Custom(EventId("DroppedFile"), Arc::new(path))
                //},
                WindowEvent::ReceivedCharacter(c) => {
                    match c {
                        // Ignore control characters
                        '\u{7f}' | // Delete
                        '\u{1b}' | // Escape
                        '\u{8}'  | // Backspace
                        '\r' | '\n' | '\t' => return,
                        _ => ()
                    };

                    queue.push_back(Input::Text(c.to_string()));
                },
                WindowEvent::Focused(focused) => queue.push_back(Input::Focus(focused)),
                WindowEvent::KeyboardInput { device_id: _, input } => {
                    queue.push_back(map_keyboard_input(&input));
                },
                WindowEvent::MouseMoved { device_id: _, position } => {
                    if capture_cursor {
                        let prev_last_cursor = *last_cursor;
                        *last_cursor = position;

                        // Don't track distance if the position is at the center, this probably is
                        //  from cursor center lock, or irrelevant.
                        if position.0 as u32 == center.0 && position.1 as u32 == center.1 {
                            return;
                        }

                        // Add the distance to the tracked cursor movement
                        cursor_accumulator.0 += position.0 - prev_last_cursor.0 as f64;
                        cursor_accumulator.1 += position.1 - prev_last_cursor.1 as f64;

                        return;
                    } else {
                        queue.push_back(Input::Move(Motion::MouseCursor(position.0, position.1)));
                    }
                },
                WindowEvent::MouseEntered { device_id: _ } =>
                    queue.push_back(Input::Cursor(true)),
                WindowEvent::MouseLeft { device_id: _ } =>
                    queue.push_back(Input::Cursor(false)),
                WindowEvent::MouseWheel { device_id: _, delta, phase: _ } => {
                    queue.push_back(match delta {
                        MouseScrollDelta::PixelDelta(x, y) =>
                            Input::Move(Motion::MouseScroll(x as f64, y as f64)),
                        MouseScrollDelta::LineDelta(x, y) =>
                            Input::Move(Motion::MouseScroll(x as f64, y as f64)),
                    });
                },
                WindowEvent::MouseInput { device_id: _, state, button } => {
                    let button = map_mouse_button(button);
                    let state = if state == ElementState::Pressed {
                        ButtonState::Press
                    } else {
                        ButtonState::Release
                    };

                    queue.push_back(Input::Button(ButtonArgs {
                        state: state,
                        button: Button::Mouse(button),
                        scancode: None,
                    }));
                },
                _ => (),
            }
        },
        _ => (),
    }
}

fn map_keyboard_input(input: &KeyboardInput) -> Input {
    use winit::VirtualKeyCode::*;
    // TODO: Complete the lookup match
    let key = if let Some(vk) = input.virtual_keycode {
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
            LMenu => Key::Menu,
            LShift => Key::LShift,

            RAlt => Key::LAlt,
            RControl => Key::RCtrl,
            RMenu => Key::Menu,
            RShift => Key::RShift,

            Tab => Key::Tab,
            _ => Key::Unknown,
        }
    } else {
        Key::Unknown
    };

    let state = if input.state == ElementState::Pressed {
        ButtonState::Press
    } else {
        ButtonState::Release
    };

    Input::Button(ButtonArgs {
        state: state,
        button: Button::Keyboard(key),
        scancode: Some(input.scancode as i32),
    })
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

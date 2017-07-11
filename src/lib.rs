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
use winit::{EventsLoop, WindowBuilder, Event as WinitEvent, WindowEvent, ElementState, MouseButton as WinitMouseButton, KeyboardInput, VirtualKeyCode};
use input::{Input, EventId, CloseArgs, Motion, Button, MouseButton, Key};
use window::{Window, Size};

pub fn required_extensions() -> InstanceExtensions {
    vulkano_win::required_extensions()
}

pub struct WinitWindow {
    pub window: VulkanoWinWindow,
    pub events_loop: EventsLoop,
    pub surface: Arc<Surface>,

    size: Size,
    should_close: bool,
    queued_events: VecDeque<Input>,
}

impl WinitWindow {
    pub fn new_vulkano(instance: Arc<Instance>, title: &str, size: Size) -> Self {
        let events_loop = EventsLoop::new();
        let window = WindowBuilder::new()
            .with_dimensions(size.width, size.height)
            .with_title(title)
            .build_vk_surface(&events_loop, instance)
            .unwrap();

        let surface = window.surface().clone();

        WinitWindow {
            window,
            events_loop,
            surface,

            size,
            should_close: false,
            queued_events: VecDeque::new(),
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
        // TODO: Report outer size rather than inner size
        self.size
    }

    fn swap_buffers(&mut self) {
        // TODO: Unclear what to do here, we don't have buffers to swap
        unimplemented!()
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
        // Add all events we got to the event queue, since winit only allows us to get all pending
        //  events at once.
        {
            let queued_events = &mut self.queued_events;
            self.events_loop.poll_events(|event| {
                push_events_for(event, queued_events)
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
        self.size
    }
}

fn push_events_for(event: WinitEvent, queue: &mut VecDeque<Input>) {
    let unsupported_input = Input::Custom(EventId("Unsupported Winit Event"), Arc::new(0));

    let event = match event {
        WinitEvent::WindowEvent { event: ev, .. } => {
            match ev {
                WindowEvent::Closed => Input::Close(CloseArgs),
                WindowEvent::KeyboardInput { device_id: _, input } => {
                    // We also need to add text events in this special case
                    push_text_for(&input, queue);
                    map_keyboard_input(&input)
                },
                WindowEvent::MouseMoved { device_id: _, position } =>
                    Input::Move(Motion::MouseCursor(position.0, position.1)),
                WindowEvent::MouseInput { device_id: _, state, button } => {
                    let button = map_mouse_button(button);
                    if state == ElementState::Pressed {
                        Input::Press(Button::Mouse(button))
                    } else {
                        Input::Release(Button::Mouse(button))
                    }
                },
                _ => unsupported_input,
            }
        },
        _ => unsupported_input,
    };

    queue.push_back(event);
}

fn push_text_for(input: &KeyboardInput, queue: &mut VecDeque<Input>) {
    // If we're releasing we don't need to do anything
    if input.state == ElementState::Released { return }

    // If we don't have a virtual keycode, we can't push any text
    let vk = if let Some(vk) = input.virtual_keycode { vk } else { return };

    // Now that we have a keycode, figure out what the actual character for it is
    let c = map_char_for_vk(vk, input.modifiers.shift);

    // If we got a character, add an event for it
    if let Some(c) = c {
        queue.push_back(Input::Text(c.to_string()));
    }
}

fn map_char_for_vk(vk: VirtualKeyCode, shift: bool) -> Option<char> {
    // TODO: Complete the lookup match
    use winit::VirtualKeyCode::*;
    let c = match vk {
        Key1 => '1',
        Key2 => '2',
        Key3 => '3',
        Key4 => '4',
        Key5 => '5',
        Key6 => '6',
        Key7 => '7',
        Key8 => '8',
        Key9 => '9',
        Key0 => '0',
        A => 'a',
        B => 'b',
        C => 'c',
        D => 'd',
        E => 'e',
        F => 'f',
        G => 'g',
        H => 'h',
        I => 'i',
        J => 'j',
        K => 'k',
        L => 'l',
        M => 'm',
        N => 'n',
        O => 'o',
        P => 'p',
        Q => 'q',
        R => 'r',
        S => 's',
        T => 't',
        U => 'u',
        V => 'v',
        W => 'w',
        X => 'x',
        Y => 'y',
        Z => 'z',
        Space => ' ',
        _ => '😞', // shush it's a valid strategy
    };

    if c == '😞' {
        None
    } else {
        Some(if shift { c.to_uppercase().next().unwrap() } else { c })
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
            Tab => Key::Tab,
            _ => Key::Unknown,
        }
    } else {
        Key::Unknown
    };

    if input.state == ElementState::Pressed {
        Input::Press(Button::Keyboard(key))
    } else {
        Input::Release(Button::Keyboard(key))
    }
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

use crate::{map_key, map_keyboard_input, map_mouse_button, UserEvent};
use input::{Button, ButtonArgs, ButtonState, CloseArgs, Event, Input, Key, Motion, ResizeArgs};
use std::{collections::VecDeque, sync::Arc, time::Duration};
#[cfg(feature = "use-vulkano")]
use vulkano::{instance::Instance, swapchain::Surface};
use window::{AdvancedWindow, Position, Size, Window, WindowSettings};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{ElementState, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

pub use vulkano_win::required_extensions;

pub struct VulkanoWindow {
    // TODO: These public fields should be changed to accessors
    pub event_loop: EventLoop<UserEvent>,
    surface: Arc<Surface<winit::window::Window>>,

    should_close: bool,
    queued_events: VecDeque<Event>,
    last_cursor: LogicalPosition<f64>,
    cursor_accumulator: LogicalPosition<f64>,

    title: String,
    capture_cursor: bool,
    exit_on_esc: bool,
}

impl VulkanoWindow {
    pub fn new(instance: Arc<Instance>, settings: &WindowSettings) -> Self {
        use vulkano_win::VkSurfaceBuild;

        let event_loop = EventLoop::with_user_event();
        let surface = WindowBuilder::new()
            .with_inner_size(LogicalSize::<f64>::new(
                settings.get_size().width.into(),
                settings.get_size().height.into(),
            ))
            .with_title(settings.get_title())
            .build_vk_surface(&event_loop, instance)
            .unwrap();

        VulkanoWindow {
            surface,
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
        self.surface.window()
    }
    fn handle_event<T>(&mut self, event: winit::event::Event<T>, center: PhysicalPosition<f64>) {
        match event {
            winit::event::Event::WindowEvent { event: ev, .. } => {
                match ev {
                    WindowEvent::Resized(size) => self.queued_events.push_back(Event::Input(
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
                    WindowEvent::CloseRequested => self
                        .queued_events
                        .push_back(Event::Input(Input::Close(CloseArgs), None)),
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

                        self.queued_events
                            .push_back(Event::Input(Input::Text(c.to_string()), None));
                    }
                    WindowEvent::Focused(focused) => self
                        .queued_events
                        .push_back(Event::Input(Input::Focus(focused), None)),
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input,
                        is_synthetic: _,
                    } => {
                        let key = map_key(&input);
                        if self.exit_on_esc && key == Key::Escape {
                            self.set_should_close(true);
                        } else {
                            self.queued_events.push_back(map_keyboard_input(&input));
                        }
                    }
                    #[allow(deprecated)]
                    WindowEvent::CursorMoved {
                        device_id: _,
                        position,
                        modifiers: _,
                    } => {
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
                        } else {
                            self.queued_events.push_back(Event::Input(
                                Input::Move(Motion::MouseCursor([position.x, position.y])),
                                None,
                            ));
                        }
                    }
                    WindowEvent::CursorEntered { device_id: _ } => self
                        .queued_events
                        .push_back(Event::Input(Input::Cursor(true), None)),
                    WindowEvent::CursorLeft { device_id: _ } => self
                        .queued_events
                        .push_back(Event::Input(Input::Cursor(false), None)),
                    #[allow(deprecated)]
                    WindowEvent::MouseWheel {
                        device_id: _,
                        delta,
                        phase: _,
                        modifiers: _,
                    } => {
                        self.queued_events.push_back(match delta {
                            MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => {
                                Event::Input(
                                    Input::Move(Motion::MouseScroll([x as f64, y as f64])),
                                    None,
                                )
                            }
                            MouseScrollDelta::LineDelta(x, y) => Event::Input(
                                Input::Move(Motion::MouseScroll([x as f64, y as f64])),
                                None,
                            ),
                        });
                    }
                    #[allow(deprecated)]
                    WindowEvent::MouseInput {
                        device_id: _,
                        state,
                        button,
                        modifiers: _,
                    } => {
                        let button = map_mouse_button(button);
                        let state = if state == ElementState::Pressed {
                            ButtonState::Press
                        } else {
                            ButtonState::Release
                        };

                        self.queued_events.push_back(Event::Input(
                            Input::Button(ButtonArgs {
                                state: state,
                                button: Button::Mouse(button),
                                scancode: None,
                            }),
                            None,
                        ));
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

impl Window for VulkanoWindow {
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

impl AdvancedWindow for VulkanoWindow {
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

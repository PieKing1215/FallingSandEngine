use glutin::{event::{
    ElementState, KeyboardInput, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent,
}, dpi::PhysicalPosition};

#[derive(Debug)]
pub enum InputEvent<'a> {
    GlutinEvent(&'a WindowEvent<'a>),
}

// TODO: make/use a new fn instead
pub struct Controls {
    pub cur_modifiers: ModifiersState,
    pub cursor_pos: PhysicalPosition<f64>,

    pub up: Box<dyn Control<bool>>,
    pub down: Box<dyn Control<bool>>,
    pub left: Box<dyn Control<bool>>,
    pub right: Box<dyn Control<bool>>,

    pub jump: Box<dyn Control<bool>>,
    pub launch: Box<dyn Control<bool>>,
    pub grapple: Box<dyn Control<bool>>,

    pub free_fly: Box<dyn Control<bool>>,

    pub copy: Box<dyn Control<bool>>,
    pub cut: Box<dyn Control<bool>>,
    pub paste: Box<dyn Control<bool>>,
    pub clipboard_action: Box<dyn Control<bool>>,
}

impl Controls {
    pub fn process(&mut self, event: &InputEvent) {
        if let InputEvent::GlutinEvent(glutin::event::WindowEvent::ModifiersChanged(modifiers)) =
            event
        {
            self.cur_modifiers = *modifiers;
        } else if let InputEvent::GlutinEvent(glutin::event::WindowEvent::CursorMoved { position, .. }) = event {
            self.cursor_pos = *position;
        }

        self.up.process(event, &self.cur_modifiers);
        self.down.process(event, &self.cur_modifiers);
        self.left.process(event, &self.cur_modifiers);
        self.right.process(event, &self.cur_modifiers);

        self.jump.process(event, &self.cur_modifiers);
        self.launch.process(event, &self.cur_modifiers);
        self.grapple.process(event, &self.cur_modifiers);

        self.free_fly.process(event, &self.cur_modifiers);

        self.copy.process(event, &self.cur_modifiers);
        self.cut.process(event, &self.cur_modifiers);
        self.paste.process(event, &self.cur_modifiers);
        self.clipboard_action.process(event, &self.cur_modifiers);
    }
}

pub trait Control<T> {
    fn get(&mut self) -> T;
    fn process(&mut self, event: &InputEvent, modifiers: &ModifiersState);
}

impl<T: Control<bool>> Control<f32> for T {
    fn get(&mut self) -> f32 {
        if T::get(self) {
            1.0
        } else {
            0.0
        }
    }

    fn process(&mut self, event: &InputEvent, modifiers: &ModifiersState) {
        T::process(self, event, modifiers);
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum KeyControlMode {
    Momentary,
    Rising,
    Falling,
    Toggle,
    Type,
}

pub struct KeyControl {
    pub key: VirtualKeyCode,
    pub mode: KeyControlMode,
    pub modifiers: ModifiersState,

    raw: bool,
    last_raw: bool,
    last_state: bool,
}

impl KeyControl {
    pub fn new(key: VirtualKeyCode, mode: KeyControlMode, modifiers: ModifiersState) -> Self {
        Self {
            key,
            mode,
            modifiers,
            raw: false,
            last_raw: false,
            last_state: false,
        }
    }
}

impl Control<bool> for KeyControl {
    fn get(&mut self) -> bool {
        let ret = match self.mode {
            KeyControlMode::Momentary => self.raw,
            KeyControlMode::Rising => self.raw && !self.last_raw,
            KeyControlMode::Falling => !self.raw && self.last_raw,
            KeyControlMode::Toggle => {
                if self.raw && self.last_raw {
                    self.last_state = !self.last_state;
                }
                self.last_state
            },
            KeyControlMode::Type => {
                let r = self.raw;
                self.raw = false;
                r
            },
        };

        self.last_raw = self.raw;

        ret
    }

    fn process(&mut self, event: &InputEvent, modifiers: &ModifiersState) {
        // log::debug!("{:?}", event);
        #[allow(clippy::match_wildcard_for_single_variants)]
        match event {
            InputEvent::GlutinEvent(glutin::event::WindowEvent::KeyboardInput {
                input: KeyboardInput { state, virtual_keycode: Some(k), .. },
                ..
            }) if *k == self.key => {
                // if !repeat || self.mode == KeyControlMode::Type {
                self.raw = *state == ElementState::Pressed && modifiers.contains(self.modifiers);
                // }
            },
            _ => {},
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub enum MouseButtonControlMode {
    Momentary,
    Rising,
    Falling,
    Toggle,
}

pub struct MouseButtonControl {
    pub button: MouseButton,
    pub mode: MouseButtonControlMode,
    pub modifiers: ModifiersState,

    raw: bool,
    last_raw: bool,
    last_state: bool,
}

impl MouseButtonControl {
    pub fn new(
        button: MouseButton,
        mode: MouseButtonControlMode,
        modifiers: ModifiersState,
    ) -> Self {
        Self {
            button,
            mode,
            modifiers,
            raw: false,
            last_raw: false,
            last_state: false,
        }
    }
}

impl Control<bool> for MouseButtonControl {
    fn get(&mut self) -> bool {
        let ret = match self.mode {
            MouseButtonControlMode::Momentary => self.raw,
            MouseButtonControlMode::Rising => self.raw && !self.last_raw,
            MouseButtonControlMode::Falling => !self.raw && self.last_raw,
            MouseButtonControlMode::Toggle => {
                if self.raw && self.last_raw {
                    self.last_state = !self.last_state;
                }
                self.last_state
            },
        };

        self.last_raw = self.raw;

        ret
    }

    fn process(&mut self, event: &InputEvent, modifiers: &ModifiersState) {
        match event {
            InputEvent::GlutinEvent(glutin::event::WindowEvent::MouseInput {
                state,
                button,
                ..
            }) if *button == self.button => {
                self.raw = *state == ElementState::Pressed && modifiers.contains(self.modifiers);
            },
            _ => {},
        }
    }
}

#[allow(dead_code)]
pub enum MultiControlMode {
    And,
    Or,
}

pub struct MultiControl {
    pub mode: MultiControlMode,
    pub controls: Vec<Box<dyn Control<bool>>>,
}

impl MultiControl {
    pub fn new(mode: MultiControlMode, controls: Vec<Box<dyn Control<bool>>>) -> Self {
        Self { mode, controls }
    }
}

impl Control<bool> for MultiControl {
    fn get(&mut self) -> bool {
        match self.mode {
            MultiControlMode::And => self.controls.iter_mut().all(|c| c.get()),
            MultiControlMode::Or => self.controls.iter_mut().any(|c| c.get()),
        }
    }

    fn process(&mut self, event: &InputEvent, modifiers: &ModifiersState) {
        self.controls
            .iter_mut()
            .for_each(|c| c.process(event, modifiers));
    }
}

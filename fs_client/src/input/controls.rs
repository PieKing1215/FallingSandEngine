use glutin::event::{WindowEvent, VirtualKeyCode, KeyboardInput, ElementState};

#[derive(Debug)]
pub enum InputEvent<'a> {
    GlutinEvent(&'a WindowEvent<'a>),
}

pub struct Controls {
    pub up: Box<dyn Control<bool>>,
    pub down: Box<dyn Control<bool>>,
    pub left: Box<dyn Control<bool>>,
    pub right: Box<dyn Control<bool>>,

    pub jump: Box<dyn Control<bool>>,
    pub launch: Box<dyn Control<bool>>,
    pub grapple: Box<dyn Control<bool>>,

    pub free_fly: Box<dyn Control<bool>>,
}

impl Controls {
    pub fn process(&mut self, event: &InputEvent) {
        self.up.process(event);
        self.down.process(event);
        self.left.process(event);
        self.right.process(event);

        self.jump.process(event);
        self.launch.process(event);
        self.grapple.process(event);

        self.free_fly.process(event);
    }
}

pub trait Control<T> {
    fn get(&mut self) -> T;
    fn process(&mut self, event: &InputEvent);
}

impl<T: Control<bool>> Control<f32> for T {
    fn get(&mut self) -> f32 {
        if T::get(self) {
            1.0
        } else {
            0.0
        }
    }

    fn process(&mut self, event: &InputEvent) {
        T::process(self, event);
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
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

    raw: bool,
    last_raw: bool,
    last_state: bool,
}

impl KeyControl {
    pub fn new(key: VirtualKeyCode, mode: KeyControlMode) -> Self {
        Self {
            key,
            mode,
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

    fn process(&mut self, event: &InputEvent) {
        // log::debug!("{:?}", event);
        #[allow(clippy::match_wildcard_for_single_variants)]
        match event {
            InputEvent::GlutinEvent(glutin::event::WindowEvent::KeyboardInput {
                input: KeyboardInput { scancode, state, virtual_keycode: Some(k), .. }, ..
            }) if *k == self.key => {
                // if !repeat || self.mode == KeyControlMode::Type {
                    self.raw = *state == ElementState::Pressed;
                // }
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

    fn process(&mut self, event: &InputEvent) {
        self.controls.iter_mut().for_each(|c| c.process(event));
    }
}

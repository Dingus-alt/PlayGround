//! Input state, normalized between keyboard, mouse and touch.

use crate::math::V2;

pub const KEY_A: usize = 0;
pub const KEY_D: usize = 1;
pub const KEY_S: usize = 2;
pub const KEY_W: usize = 3;
pub const KEY_SPACE: usize = 4;
pub const KEY_SHIFT: usize = 5;
pub const KEY_E: usize = 6;
pub const KEY_C: usize = 7;
pub const KEY_M: usize = 8;
pub const KEY_ESC: usize = 9;
pub const KEY_DIGIT_1: usize = 10;
pub const KEY_DIGIT_0: usize = 19;
pub const KEY_N: usize = 20;
pub const KEY_ENTER: usize = 21;
pub const KEY_LEFT: usize = 22;
pub const KEY_RIGHT: usize = 23;
pub const KEY_UP: usize = 24;
pub const KEY_DOWN: usize = 25;
pub const KEY_COUNT: usize = 26;

#[derive(Clone, Debug)]
pub struct Input {
    pub keys: [bool; KEY_COUNT],
    pub pressed: [bool; KEY_COUNT],
    pub mouse: V2,
    pub mouse_down: bool,
    pub mouse_pressed: bool,
    pub mouse_released: bool,
    pub right_down: bool,
    pub right_pressed: bool,
    pub right_released: bool,
    pub wheel: f32,
    pub touch_mode: bool,
    pub touches: Vec<Touch>,
    pub any_input: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Touch {
    pub id: u32,
    pub pos: V2,
    pub active: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Input {
    pub fn new() -> Self {
        Input {
            keys: [false; KEY_COUNT],
            pressed: [false; KEY_COUNT],
            mouse: V2::new(-100.0, -100.0),
            mouse_down: false,
            mouse_pressed: false,
            mouse_released: false,
            right_down: false,
            right_pressed: false,
            right_released: false,
            wheel: 0.0,
            touch_mode: false,
            touches: Vec::new(),
            any_input: false,
        }
    }

    pub fn end_frame(&mut self) {
        self.pressed = [false; KEY_COUNT];
        self.mouse_pressed = false;
        self.mouse_released = false;
        self.right_pressed = false;
        self.right_released = false;
        self.wheel = 0.0;
        self.any_input = false;
    }

    pub fn key(&self, k: usize) -> bool {
        self.keys.get(k).copied().unwrap_or(false)
    }

    pub fn press(&self, k: usize) -> bool {
        self.pressed.get(k).copied().unwrap_or(false)
    }

    pub fn left(&self) -> bool {
        self.key(KEY_A) || self.key(KEY_LEFT)
    }

    pub fn right(&self) -> bool {
        self.key(KEY_D) || self.key(KEY_RIGHT)
    }

    pub fn up(&self) -> bool {
        self.key(KEY_W) || self.key(KEY_UP)
    }

    pub fn down(&self) -> bool {
        self.key(KEY_S) || self.key(KEY_DOWN)
    }

    pub fn jump_held(&self) -> bool {
        self.key(KEY_SPACE) || self.key(KEY_W) || self.key(KEY_UP)
    }

    pub fn jump_pressed(&self) -> bool {
        self.press(KEY_SPACE) || self.press(KEY_W) || self.press(KEY_UP)
    }

    pub fn down_pressed(&self) -> bool {
        self.press(KEY_S) || self.press(KEY_DOWN)
    }
}

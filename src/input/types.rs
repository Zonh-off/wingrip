#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    Press,
    Release,
}

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    MouseMove {
        x: i32,
        y: i32,
        shift_pressed: bool,
    },
    MouseButton {
        button: MouseButton,
        action: MouseAction,
        x: i32,
        y: i32,
    },
}

// Ver: 1 File: crate/dos/src/bat/runner/types.rs

use std::collections::HashMap;

use bus::peripherals::keyboard::Keyboard;

/// Конфигурация запуска .BAT-оболочки.
#[derive(Debug, Clone)]
pub struct BatConfig {
    pub max_depth: u16,
    pub no_stdin_keys: bool,
    pub no_log: bool,
}

/// Состояние «оболочки» между запусками программ.
#[derive(Debug, Clone, Default)]
pub struct ShellState {
    pub cwd: HashMap<char, String>,
    pub current_drive: char,
    pub env: HashMap<String, String>,
    pub echo_on: bool,
    pub errorlevel: u8,
}

/// Управление потоком выполнения внутри одного .BAT-файла.
pub(crate) enum Flow {
    Next,
    Goto(usize),
    Exit(Option<u8>),
}

/// Источник клавиш хоста (консольный мост из бинарного крейта).
pub type PumpFn<'a> = dyn FnMut(&mut Keyboard) -> Result<(), String> + 'a;

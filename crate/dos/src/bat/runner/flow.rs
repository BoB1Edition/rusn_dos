// Ver: 1 File: crate/dos/src/bat/runner/flow.rs

use std::thread;
use std::time::Duration;

use bus::Display;
use bus::peripherals::keyboard::Keyboard;

use super::types::{BatConfig, PumpFn};
use crate::system::DosSystem;

pub(crate) const NO_STDIN_MSG: &str = "--no_stdin_keys: программа ожидает ввод с клавиатуры; не используйте этот флаг с интерактивными приложениями";

/// Цикл выполнения дочерней программы. Display принимается по значению и
/// возвращается обратно: время жизни 'd живёт в вызывающем коде.
pub(crate) fn child_loop<'d>(
    ds: &mut DosSystem,
    mut display: Option<&'d mut dyn Display>,
    pump: &mut PumpFn,
    cfg: &BatConfig,
) -> (Result<u8, String>, Option<&'d mut dyn Display>) {
    let result = loop {
        if ds.cpu.halted || ds.program_terminated {
            break Ok(if ds.program_terminated {
                ds.exit_code
            } else {
                1
            });
        }
        if let Err(e) = pump(&mut ds.machine.keyboard) {
            break Err(e);
        }
        // Явный ре-бороу строго внутри аргумента вызова: заимствование живёт
        // ровно до конца step_frame и НЕ растягивается на 'd / на итерации цикла.
        match display.as_mut() {
            Some(d) => ds.step_frame(Some(&mut **d)),
            None => ds.step_frame(None),
        }
        if ds.suspended.is_some() {
            if cfg.no_stdin_keys && display.is_none() {
                break Err(NO_STDIN_MSG.to_string());
            }
            if display.is_none() {
                thread::sleep(Duration::from_millis(10));
            }
        }
    };
    let result = if display.as_ref().map(|d| !d.is_open()).unwrap_or(false) {
        Err("Окно эмулятора закрыто во время работы дочерней программы".into())
    } else {
        result
    };
    (result, display)
}

/// Ожидание клавиши между детьми (pause): консоль через pump, графика через окно.
pub(crate) fn wait_key<'d>(
    mut display: Option<&'d mut dyn Display>,
    pump: &mut PumpFn,
) -> (Result<(), String>, Option<&'d mut dyn Display>) {
    let mut kb = Keyboard::new();
    let result = loop {
        if let Err(e) = pump(&mut kb) {
            break Err(e);
        }
        // То же правило: borrow только на время вызова poll_keys.
        let keys: Vec<(u8, u8)> = match display.as_mut() {
            Some(d) => d.poll_keys(),
            None => Vec::new(),
        };
        for (sc, ch) in keys {
            kb.push_key(sc, ch);
        }
        if kb.pop_key().is_some() {
            break Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    };
    (result, display)
}

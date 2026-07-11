// Ver: 6 File: ./libs/dos_core/src/cpu/run.rs

use crate::{DosMachine, cpu::execute_0f::execute_0f, executor::execute, video};
use minifb::Key;
use std::{collections::HashSet, error::Error};

fn minifb_to_dos(key: Key) -> Option<(u8, u8)> {
    match key {
        Key::A => Some((0x1E, b'a')),
        Key::B => Some((0x30, b'b')),
        Key::C => Some((0x2E, b'c')),
        Key::D => Some((0x20, b'd')),
        Key::E => Some((0x12, b'e')),
        Key::F => Some((0x21, b'f')),
        Key::G => Some((0x22, b'g')),
        Key::H => Some((0x23, b'h')),
        Key::I => Some((0x17, b'i')),
        Key::J => Some((0x24, b'j')),
        Key::K => Some((0x25, b'k')),
        Key::L => Some((0x26, b'l')),
        Key::M => Some((0x32, b'm')),
        Key::N => Some((0x31, b'n')),
        Key::O => Some((0x18, b'o')),
        Key::P => Some((0x19, b'p')),
        Key::Q => Some((0x10, b'q')),
        Key::R => Some((0x13, b'r')),
        Key::S => Some((0x1F, b's')),
        Key::T => Some((0x14, b't')),
        Key::U => Some((0x16, b'u')),
        Key::V => Some((0x2F, b'v')),
        Key::W => Some((0x11, b'w')),
        Key::X => Some((0x2D, b'x')),
        Key::Y => Some((0x15, b'y')),
        Key::Z => Some((0x2C, b'z')),
        Key::Key0 => Some((0x0B, b'0')),
        Key::Key1 => Some((0x02, b'1')),
        Key::Key2 => Some((0x03, b'2')),
        Key::Key3 => Some((0x04, b'3')),
        Key::Key4 => Some((0x05, b'4')),
        Key::Key5 => Some((0x06, b'5')),
        Key::Key6 => Some((0x07, b'6')),
        Key::Key7 => Some((0x08, b'7')),
        Key::Key8 => Some((0x09, b'8')),
        Key::Key9 => Some((0x0A, b'9')),
        Key::Minus => Some((0x0C, b'-')),
        Key::Equal => Some((0x0D, b'=')),
        Key::LeftBracket => Some((0x1A, b'[')),
        Key::RightBracket => Some((0x1B, b']')),
        Key::Semicolon => Some((0x27, b';')),
        Key::Apostrophe => Some((0x28, b'\'')),
        Key::Backslash => Some((0x2B, b'\\')),
        Key::Comma => Some((0x33, b',')),
        Key::Period => Some((0x34, b'.')),
        Key::Slash => Some((0x35, b'/')),
        Key::Enter => Some((0x1C, 0x0D)),
        Key::Escape => Some((0x01, 0x1B)),
        Key::Backspace => Some((0x0E, 0x08)),
        Key::Tab => Some((0x0F, 0x09)),
        Key::Space => Some((0x39, b' ')),
        // Стрелки и навигация (ASCII = 0, только scancode)
        Key::Up => Some((0x48, 0)),
        Key::Down => Some((0x50, 0)),
        Key::Left => Some((0x4B, 0)),
        Key::Right => Some((0x4D, 0)),
        Key::Home => Some((0x47, 0)),
        Key::End => Some((0x4F, 0)),
        Key::PageUp => Some((0x49, 0)),
        Key::PageDown => Some((0x51, 0)),
        Key::Insert => Some((0x52, 0)),
        Key::Delete => Some((0x53, 0)),
        // F1-F10
        Key::F1 => Some((0x3B, 0)),
        Key::F2 => Some((0x3C, 0)),
        Key::F3 => Some((0x3D, 0)),
        Key::F4 => Some((0x3E, 0)),
        Key::F5 => Some((0x3F, 0)),
        Key::F6 => Some((0x40, 0)),
        Key::F7 => Some((0x41, 0)),
        Key::F8 => Some((0x42, 0)),
        Key::F9 => Some((0x43, 0)),
        Key::F10 => Some((0x44, 0)),
        // Модификаторы
        Key::LeftShift | Key::RightShift => Some((0x2A, 0)),
        Key::LeftCtrl | Key::RightCtrl => Some((0x1D, 0)),
        Key::LeftAlt | Key::RightAlt => Some((0x38, 0)),
        _ => None,
    }
}

pub(crate) fn run(machine: &mut DosMachine) -> Result<Option<u8>, Box<dyn Error>> {
    let mut tick_counter: u64 = 65535;
    let mut prev_keys: HashSet<Key> = HashSet::new();

    while !machine.halted {
        let opcode = machine.read_instr_u8(machine.registers.ip());
        machine.registers.step(None);

        match opcode {
            0x0F => machine.has_extended_prefix = true,
            0x67 => machine.has_address_size_prefix = true,
            0x66 => machine.has_operand_size_prefix = true,
            0x26 => {
                machine.override_segment = Some(machine.registers.es());
                machine.opcode_override_segment = Some(opcode);
            }
            0x2E => {
                machine.override_segment = Some(machine.registers.cs());
                machine.opcode_override_segment = Some(opcode);
            }
            0x36 => {
                machine.override_segment = Some(machine.registers.ss());
                machine.opcode_override_segment = Some(opcode);
            }
            0x3E => {
                machine.override_segment = Some(machine.registers.ds());
                machine.opcode_override_segment = Some(opcode);
            }
            0x64 => {
                machine.override_segment = Some(machine.registers.fs());
                machine.opcode_override_segment = Some(opcode);
            }
            0x65 => {
                machine.override_segment = Some(machine.registers.gs());
                machine.opcode_override_segment = Some(opcode);
            }
            0xF0 => {
                machine.has_lock_prefix = true;
                machine.rep_prefix_type = Some(0xF0)
            }
            0xF2 => {
                machine.has_rep_prefix = true;
                machine.rep_prefix_type = Some(0xF2)
            }
            0xF3 => {
                machine.has_rep_prefix = true;
                machine.rep_prefix_type = Some(0xF3)
            }
            _ => {
                if machine.has_extended_prefix {
                    execute_0f(machine, opcode);
                } else {
                    execute(machine, opcode);
                }
                machine.has_address_size_prefix = false;
                machine.has_operand_size_prefix = false;
                machine.has_extended_prefix = false;
                machine.has_rep_prefix = false;
                machine.has_lock_prefix = false;
                machine.override_segment = None;
                machine.rep_prefix_type = None;
                machine.opcode_override_segment = None;
            }
        }

        if let Some(window) = machine.window() {
            if !window.is_open() {
                machine.halted = true;
                break;
            }

            let current_keys: HashSet<Key> = window.get_keys().into_iter().collect();

            // Находим только НОВЫЕ нажатия (есть в current, нет в prev)
            for key in current_keys.difference(&prev_keys) {
                if let Some((scancode, ascii)) = minifb_to_dos(*key) {
                    machine.keyboard.push_key(scancode, ascii);
                }
            }
            prev_keys = current_keys;
        }

        if machine.video.dirty {
            let (src_buffer, src_width, src_height) = match machine.video.mode {
                crate::video::VideoMode::Text80x25 => {
                    let buf = crate::video::render_text_to_pixels(
                        &machine.video.text_buffer.data,
                        &crate::video::get_fonts_vga8x16(),
                    );
                    (buf, 640, 400)
                }
                crate::video::VideoMode::Mode13h => {
                    if let Some(fb) = &machine.video.framebuffer {
                        let buf = crate::video::upscale_framebuffer(
                            &fb.data,
                            &crate::video::load_vga_palette(),
                        );
                        (buf, 1920, 1200)
                    } else {
                        (vec![0u32; 640 * 400], 640, 400)
                    }
                }
            };

            if let Some(window) = machine.window() {
                let (dst_width, dst_height) = window.get_size();

                let scaled_buffer = if src_width != dst_width || src_height != dst_height {
                    crate::video::scale_buffer(
                        &src_buffer,
                        src_width,
                        src_height,
                        dst_width,
                        dst_height,
                    )
                } else {
                    src_buffer
                };

                window
                    .update_with_buffer(&scaled_buffer, dst_width, dst_height)
                    .unwrap();
            }

            machine.video.dirty = false;
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        // === Обработка системного таймера (IRQ0) ===
        tick_counter += 1;
        if tick_counter >= 65536 {
            if !machine.inhibit_interrupts && crate::flags::test_if(machine.registers.flags()) {
                crate::instructions::system::call_interrupt(machine, 0x08);
            }
            tick_counter = 0;
        }

        // Сброс флага inhibit_interrupts (если был установлен)
        if machine.inhibit_interrupts {
            machine.inhibit_interrupts = false;
        }
    }

    Ok(Some(machine.registers.al()))
}

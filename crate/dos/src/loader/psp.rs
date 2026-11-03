// Ver: 6 File: crate/dos/src/loader/psp.rs
use crate::consts::{MCB_SIGNATURE_LAST, MCB_SIGNATURE_NON_LAST};
use crate::mcb::MCB;
use bus::Machine;
use emu_config::EnvironmentConfig;

/// Создает блок окружения (Environment Block).
pub fn build_environment_block(
    machine: &mut dyn Machine,
    psp_segment: u16,
    env: &EnvironmentConfig,
) -> u16 {
    let mut pairs: Vec<(&String, &String)> = env.variables.iter().collect();
    pairs.sort_by_key(|(k, _)| k.to_ascii_uppercase());
    let mut bytes: Vec<u8> = Vec::new();
    for (k, v) in pairs {
        bytes.extend_from_slice(k.as_bytes());
        bytes.push(b'=');
        bytes.extend_from_slice(v.as_bytes());
        bytes.push(0);
    }
    bytes.push(0); // терминальная пустая строка
    let paragraphs = ((bytes.len() + 15) / 16).max(1) as u16;
    let env_segment = psp_segment.wrapping_sub(1).wrapping_sub(paragraphs);
    MCB {
        signature: MCB_SIGNATURE_NON_LAST,
        owner: psp_segment,
        size: paragraphs,
    }
    .write(machine, env_segment.wrapping_sub(1));
    let env_base = (env_segment as u32) << 4;
    for (i, &b) in bytes.iter().enumerate() {
        machine.write_mem_u8(env_base + i as u32, b);
    }
    log::info!(
        "Environment block at {:04X}: {} bytes, {} paragraphs",
        env_segment,
        bytes.len(),
        paragraphs
    );
    env_segment
}

fn fill_fcb(machine: &mut dyn Machine, psp_base: u32, fcb_off: u32, name: &str, ext: &str) {
    machine.write_mem_u8(psp_base + fcb_off, 0x00); // drive = default
    for i in 0..8 {
        let c = name.chars().nth(i).unwrap_or(' ').to_ascii_uppercase() as u8;
        machine.write_mem_u8(psp_base + fcb_off + 1 + i as u32, c);
    }
    for i in 0..3 {
        let c = ext.chars().nth(i).unwrap_or(' ').to_ascii_uppercase() as u8;
        machine.write_mem_u8(psp_base + fcb_off + 9 + i as u32, c);
    }
}

/// Формирует полноценный PSP (Program Segment Prefix).
pub fn build_psp(
    machine: &mut dyn Machine,
    psp_segment: u16,
    env_segment: u16,
    mem_paragraphs: u16,
    args: &[String],
) {
    let psp_base = (psp_segment as u32) << 4;

    // 1. INT 20h
    machine.write_mem_u8(psp_base, 0xCD);
    machine.write_mem_u8(psp_base + 1, 0x20);
    // 2. Размер доступной памяти (в параграфах)
    machine.write_mem_u16(psp_base + 0x02, mem_paragraphs);
    // 3. INT 21h / RETF
    machine.write_mem_u8(psp_base + 5, 0xCD);
    machine.write_mem_u8(psp_base + 6, 0x21);
    machine.write_mem_u8(psp_base + 7, 0xCB);
    // 4. Сегмент окружения
    machine.write_mem_u16(psp_base + 0x2C, env_segment);

    // 5. FCB1 (5Ch) и FCB2 (6Ch) — ПЕРЕД tail и только 12 байт каждый
    let (name1, ext1) = match args.first() {
        Some(a) => {
            let arg = a.trim_matches('"');
            let mut parts = arg.split('.');
            let n = parts.next().unwrap_or("");
            let e = parts.next().unwrap_or("");
            (n.to_string(), e.to_string())
        }
        None => (String::new(), String::new()),
    };
    fill_fcb(machine, psp_base, 0x5C, &name1, &ext1);
    fill_fcb(machine, psp_base, 0x6C, "", "");

    // 6. Command Tail по смещению 0x80 (пишется ПОСЛЕ FCB, длина без CR)
    let mut cmd_tail = String::new();
    for arg in args {
        cmd_tail.push(' ');
        cmd_tail.push_str(arg);
    }
    if cmd_tail.len() > 126 {
        cmd_tail.truncate(126);
    }
    let len = cmd_tail.len() as u8;
    machine.write_mem_u8(psp_base + 0x80, len);
    for (i, &byte) in cmd_tail.as_bytes().iter().enumerate() {
        machine.write_mem_u8(psp_base + 0x81 + i as u32, byte);
    }
    machine.write_mem_u8(psp_base + 0x81 + len as u32, 0x0D);

    log::info!(
        "🔍 PSP Command Tail at 0x{:04X}: len={}, bytes={:?}",
        psp_base + 0x80,
        len,
        cmd_tail.as_bytes().to_vec()
    );
}

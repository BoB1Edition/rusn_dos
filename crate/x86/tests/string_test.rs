// Ver: 1 File: crate/x86/tests/string_test.rs
use bus::{Cpu, Machine, Motherboard};
use x86::cpu::X86Cpu;
use x86::flags;

fn boot(bytes: &[u8]) -> (X86Cpu, Motherboard) {
    let mut board = Motherboard::new(1024 * 1024);
    for (i, b) in bytes.iter().enumerate() {
        board.write_mem_u8(0x100 + i as u32, *b);
    }
    let mut cpu = X86Cpu::new();
    cpu.registers.set_cs(0);
    cpu.registers.set_ip(0x100);
    cpu.registers.set_ds(0);
    cpu.registers.set_es(0);
    (cpu, board)
}

#[test]
fn movsb_and_rep_and_df() {
    let (mut cpu, mut board) = boot(&[0xA4, 0xF4]);
    board.write_mem_u8(0x200, 0x41);
    cpu.registers.set_si(0x200);
    cpu.registers.set_di(0x300);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u8(0x300), 0x41);
    assert_eq!(cpu.registers.si(), 0x201);
    assert_eq!(cpu.registers.di(), 0x301);

    let (mut cpu, mut board) = boot(&[0xF3, 0xA4, 0xF4]);
    for i in 0..4u8 {
        board.write_mem_u8(0x200 + i as u32, 0x10 + i);
    }
    cpu.registers.set_si(0x200);
    cpu.registers.set_di(0x400);
    cpu.registers.set_cx(4);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cx(), 0);
    assert_eq!(board.read_mem_u8(0x403), 0x13);
    assert_eq!(cpu.registers.si(), 0x204);

    let (mut cpu, mut board) = boot(&[0xA4, 0xF4]);
    board.write_mem_u8(0x210, 0x7E);
    cpu.registers.set_si(0x210);
    cpu.registers.set_di(0x220);
    cpu.registers.set_flags(flags::DF | 0x2);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u8(0x220), 0x7E);
    assert_eq!(cpu.registers.si(), 0x20F);
    assert_eq!(cpu.registers.di(), 0x21F);
}

#[test]
fn movsd_and_stos_lods() {
    let (mut cpu, mut board) = boot(&[0x66, 0xA5, 0xF4]);
    board.write_mem_u32(0x800, 0x1122_3344);
    cpu.registers.set_si(0x800);
    cpu.registers.set_di(0x900);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u32(0x900), 0x1122_3344);
    assert_eq!(cpu.registers.si(), 0x804);

    let (mut cpu, mut board) = boot(&[0xF3, 0xAA, 0xF4]);
    cpu.registers.set_al(0xAB);
    cpu.registers.set_di(0x500);
    cpu.registers.set_cx(3);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u8(0x500), 0xAB);
    assert_eq!(board.read_mem_u8(0x502), 0xAB);
    assert_eq!(cpu.registers.di(), 0x503);
    assert_eq!(cpu.registers.cx(), 0);

    let (mut cpu, mut board) = boot(&[0xAD, 0xF4]);
    board.write_mem_u16(0x600, 0xBEEF);
    cpu.registers.set_si(0x600);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0xBEEF);
    assert_eq!(cpu.registers.si(), 0x602);
}

#[test]
fn cmps_repe_and_repne() {
    let (mut cpu, mut board) = boot(&[0xF3, 0xA6, 0xF4]);
    board.write_mem_u8(0x1000, 1);
    board.write_mem_u8(0x1001, 2);
    board.write_mem_u8(0x1002, 9);
    board.write_mem_u8(0x2000, 1);
    board.write_mem_u8(0x2001, 2);
    board.write_mem_u8(0x2002, 3);
    cpu.registers.set_si(0x1000);
    cpu.registers.set_di(0x2000);
    cpu.registers.set_cx(3);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cx(), 0);
    assert!(!flags::test_zf(cpu.registers.flags()));
    assert_eq!(cpu.registers.si(), 0x1003);

    let (mut cpu, mut board) = boot(&[0xF2, 0xAE, 0xF4]);
    board.write_mem_u8(0x300, 1);
    board.write_mem_u8(0x301, 2);
    board.write_mem_u8(0x302, 0x42);
    cpu.registers.set_al(0x42);
    cpu.registers.set_di(0x300);
    cpu.registers.set_cx(3);
    cpu.step(&mut board);
    assert!(flags::test_zf(cpu.registers.flags()));
    assert_eq!(cpu.registers.cx(), 0);
    assert_eq!(cpu.registers.di(), 0x303);
}

#[test]
fn rep_cx_zero_does_nothing() {
    let (mut cpu, mut board) = boot(&[0xF3, 0xA4, 0xF4]);
    cpu.registers.set_si(0x10);
    cpu.registers.set_di(0x20);
    cpu.registers.set_cx(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.si(), 0x10);
    assert_eq!(cpu.registers.di(), 0x20);
    assert!(!cpu.is_halted());
}

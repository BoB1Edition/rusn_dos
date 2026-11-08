// Ver: 1 File: crate/x86/tests/mov_test.rs
use bus::{Cpu, Machine, Motherboard};
use x86::cpu::X86Cpu;

fn boot(bytes: &[u8]) -> (X86Cpu, Motherboard) {
    let mut board = Motherboard::new(1024 * 1024);
    for (i, b) in bytes.iter().enumerate() {
        board.write_mem_u8(i as u32, *b);
    }
    let mut cpu = X86Cpu::new();
    cpu.registers.set_cs(0);
    cpu.registers.set_ds(0);
    cpu.registers.set_es(0);
    cpu.registers.set_ip(0);
    (cpu, board)
}

#[test]
fn mov_reg_mem_8_16_32() {
    let (mut cpu, mut board) = boot(&[0x88, 0xC1, 0x8A, 0xD0, 0x89, 0xC3, 0x8B, 0x0E, 0x00, 0x30, 0xF4]);
    cpu.registers.set_al(0x42);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cl(), 0x42);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.dl(), 0x42);
    cpu.registers.set_ax(0x1234);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bx(), 0x1234);
    board.write_mem_u16(0x3000, 0xABCD);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cx(), 0xABCD);

    let (mut cpu, mut board) = boot(&[0x66, 0x89, 0xC3, 0x66, 0x8B, 0xC3, 0xF4]);
    cpu.registers.set_eax(0x1122_3344);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ebx(), 0x1122_3344);
    cpu.registers.set_eax(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0x1122_3344);
}

#[test]
fn mov_imm8_all_and_imm16_32() {
    let (mut cpu, mut board) = boot(&[
        0xB0, 0x01, 0xB1, 0x02, 0xB2, 0x03, 0xB3, 0x04, 0xB4, 0x05, 0xB5, 0x06, 0xB6, 0x07, 0xB7,
        0x08, 0xF4,
    ]);
    for _ in 0..8 {
        cpu.step(&mut board);
    }
    assert_eq!(cpu.registers.al(), 1);
    assert_eq!(cpu.registers.cl(), 2);
    assert_eq!(cpu.registers.dl(), 3);
    assert_eq!(cpu.registers.bl(), 4);
    assert_eq!(cpu.registers.ah(), 5);
    assert_eq!(cpu.registers.ch(), 6);
    assert_eq!(cpu.registers.dh(), 7);
    assert_eq!(cpu.registers.bh(), 8);

    let (mut cpu, mut board) = boot(&[0xB8, 0x34, 0x12, 0xB9, 0x78, 0x56, 0xF4]);
    cpu.step(&mut board);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x1234);
    assert_eq!(cpu.registers.cx(), 0x5678);

    let (mut cpu, mut board) = boot(&[0x66, 0xB9, 0x44, 0x33, 0x22, 0x11, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ecx(), 0x1122_3344);
}

#[test]
fn mov_sreg_and_ss_inhibit() {
    let (mut cpu, mut board) = boot(&[0x8C, 0xD8, 0x8E, 0xD8, 0xF4]);
    cpu.registers.set_ds(0x1234);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x1234);
    cpu.registers.set_ax(0x5555);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ds(), 0x5555);

    let (mut cpu, mut board) = boot(&[0x8E, 0xD0, 0xF4]);
    cpu.registers.set_ax(0x2000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ss(), 0x2000);
    assert!(cpu.inhibit_interrupts);
}

#[test]
fn lea_and_moffs() {
    let (mut cpu, mut board) = boot(&[0x8D, 0x47, 0x02, 0xF4]); // LEA AX, [BX+2]
    cpu.registers.set_bx(0x1000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x1002);

    let (mut cpu, mut board) = boot(&[0xA0, 0x00, 0x40, 0xA2, 0x01, 0x40, 0xF4]);
    board.write_mem_u8(0x4000, 0x77);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x77);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u8(0x4001), 0x77);

    let (mut cpu, mut board) = boot(&[0x66, 0xA1, 0x10, 0x00, 0x66, 0xA3, 0x20, 0x00, 0xF4]);
    board.write_mem_u32(0x0010, 0xAABB_CCDD);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0xAABB_CCDD);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u32(0x0020), 0xAABB_CCDD);

    let (mut cpu, mut board) = boot(&[0x67, 0xA0, 0x00, 0x50, 0x00, 0x00, 0xF4]);
    board.write_mem_u8(0x5000, 0x5A);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x5A);
}

#[test]
fn les_lds_and_imm_rm() {
    let (mut cpu, mut board) = boot(&[0xC4, 0x06, 0x00, 0x60, 0xC5, 0x1E, 0x04, 0x60, 0xF4]);
    board.write_mem_u16(0x6000, 0x1111);
    board.write_mem_u16(0x6002, 0x2222);
    board.write_mem_u16(0x6004, 0x3333);
    board.write_mem_u16(0x6006, 0x4444);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x1111);
    assert_eq!(cpu.registers.es(), 0x2222);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bx(), 0x3333);
    assert_eq!(cpu.registers.ds(), 0x4444);

    let (mut cpu, mut board) = boot(&[0xC6, 0x06, 0x00, 0x70, 0x99, 0xF4]);
    cpu.step(&mut board);
    assert!(!cpu.is_halted(), "halted at ip {:04x}", cpu.registers.ip());
    assert_eq!(board.read_mem_u8(0x7000), 0x99);

    let (mut cpu, mut board) = boot(&[0xC7, 0x06, 0x02, 0x70, 0x34, 0x12, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(0x7002), 0x1234);

    let (mut cpu, mut board) = boot(&[0x66, 0xC7, 0xC0, 0x78, 0x56, 0x34, 0x12, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0x1234_5678);
}

#[test]
fn segment_override_on_moffs() {
    let (mut cpu, mut board) = boot(&[0x26, 0xA0, 0x00, 0x00, 0xF4]);
    cpu.registers.set_es(0x0100);
    board.write_mem_u8(0x1000, 0x42);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x42);
    assert!(cpu.prefixes.segment_override.is_none());
}

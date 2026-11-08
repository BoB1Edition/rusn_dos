// Ver: 1 File: crate/x86/tests/system_test.rs
use bus::{Cpu, Machine, Motherboard};
use x86::cpu::X86Cpu;
use x86::flags;

fn boot(bytes: &[u8]) -> (X86Cpu, Motherboard) {
    let mut board = Motherboard::new(64 * 1024);
    for (i, b) in bytes.iter().enumerate() {
        board.write_mem_u8(i as u32, *b);
    }
    let mut cpu = X86Cpu::new();
    cpu.registers.set_cs(0);
    cpu.registers.set_ds(0);
    cpu.registers.set_ss(0);
    cpu.registers.set_sp(0x1000);
    cpu.registers.set_ip(0);
    (cpu, board)
}

#[test]
fn cbw_cwd_and_32() {
    let (mut cpu, mut board) = boot(&[0x98, 0x99, 0xF4]);
    cpu.registers.set_al(0x80);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0xFF80);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.dx(), 0xFFFF);

    let (mut cpu, mut board) = boot(&[0x66, 0x98, 0x66, 0x99, 0xF4]);
    cpu.registers.set_ax(0x8000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0xFFFF_8000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.edx(), 0xFFFF_FFFF);
}

#[test]
fn sahf_lahf_flag_ops() {
    let (mut cpu, mut board) = boot(&[0x9E, 0x9F, 0xF5, 0xF8, 0xF9, 0xF4]);
    cpu.registers.set_ah(flags::CF as u8 | flags::ZF as u8 | 0x02);
    cpu.step(&mut board);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_zf(cpu.registers.flags()));
    cpu.registers.set_flags(flags::SF | flags::PF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ah() & 0xD5, (flags::SF | flags::PF) as u8);
    assert_eq!(cpu.registers.ah() & 0x02, 0x02);

    let (mut cpu, mut board) = boot(&[0xF5, 0xF8, 0xF9, 0xF4]);
    cpu.registers.set_flags(flags::CF | 0x2);
    cpu.step(&mut board);
    assert!(!flags::test_cf(cpu.registers.flags()));
    cpu.step(&mut board);
    assert!(!flags::test_cf(cpu.registers.flags()));
    cpu.step(&mut board);
    assert!(flags::test_cf(cpu.registers.flags()));
}

#[test]
fn int3_into_iret() {
    let (mut cpu, mut board) = boot(&[0xCC]);
    board.write_mem_u8(0x200, 0xCF);
    board.write_mem_u16(12, 0x200);
    board.write_mem_u16(14, 0x0000);
    cpu.registers.set_flags(flags::IF | flags::TF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x200);
    assert!(!flags::test_if(cpu.registers.flags()));
    assert_eq!(cpu.registers.flags() & flags::TF, 0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 1);
    assert!(flags::test_if(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xCE, 0x90, 0xF4]);
    board.write_mem_u16(16, 0x300);
    board.write_mem_u16(18, 0);
    cpu.registers.set_flags(flags::OF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x300);
}

#[test]
fn aam_aad_salc_xlat() {
    let (mut cpu, mut board) = boot(&[0xD4, 0x0A, 0xF4]);
    cpu.registers.set_al(47);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ah(), 4);
    assert_eq!(cpu.registers.al(), 7);

    let (mut cpu, mut board) = boot(&[0xD5, 0x0A, 0xF4]);
    cpu.registers.set_ah(4);
    cpu.registers.set_al(7);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 47);

    let (mut cpu, mut board) = boot(&[0xD4, 0x00]);
    board.write_mem_u16(0x200, 0x400);
    board.write_mem_u16(0x202, 0);
    cpu.registers.set_ip(0x200);
    board.write_mem_u8(0x200, 0xD4);
    board.write_mem_u8(0x201, 0x00);
    board.write_mem_u16(0, 0x400);
    board.write_mem_u16(2, 0);
    cpu.registers.set_ip(0x200);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x400);

    let (mut cpu, mut board) = boot(&[0xD6, 0xF4]);
    cpu.registers.set_flags(flags::CF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0xFF);

    let (mut cpu, mut board) = boot(&[0xD7, 0xF4]);
    cpu.registers.set_bx(0x1000);
    cpu.registers.set_al(2);
    board.write_mem_u8(0x1002, 0x5A);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x5A);
}

#[test]
fn imul_imm_and_arpl() {
    let (mut cpu, mut board) = boot(&[0x69, 0xC3, 0x02, 0x00, 0xF4]); // IMUL AX, BX, 2
    cpu.registers.set_bx(0x7FFF);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0xFFFE);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_of(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0x6B, 0xC0, 0x02, 0xF4]); // IMUL AX, AX, 2
    cpu.registers.set_ax(3);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 6);
    assert!(!flags::test_of(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0x63, 0xD8, 0xF4]); // ARPL AX, BX
    cpu.registers.set_ax(0x0000);
    cpu.registers.set_bx(0x0003);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x0003);
    assert!(flags::test_zf(cpu.registers.flags()));
}

#[test]
fn int1_halts_explicitly() {
    let (mut cpu, mut board) = boot(&[0xF1]);
    cpu.step(&mut board);
    assert!(cpu.is_halted());
}

#[test]
fn wait_and_nop() {
    let (mut cpu, mut board) = boot(&[0x9B, 0x90, 0xF4]);
    cpu.step(&mut board);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 2);
    assert!(!cpu.is_halted());
}

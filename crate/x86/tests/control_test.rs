// Ver: 1 File: crate/x86/tests/control_test.rs
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
    cpu.registers.set_ss(0);
    cpu.registers.set_sp(0x1000);
    cpu.registers.set_ip(0);
    (cpu, board)
}

#[test]
fn jcc_rel8_taken_and_not() {
    let (mut cpu, mut board) = boot(&[0x74, 0x02, 0x90, 0x90, 0xF4]);
    cpu.registers.set_flags(flags::ZF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 4);

    let (mut cpu, mut board) = boot(&[0x75, 0x02, 0x90, 0xF4]);
    cpu.registers.set_flags(flags::ZF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 2);

    let (mut cpu, mut board) = boot(&[0x7A, 0x00, 0x7B, 0x00, 0xF4]);
    cpu.registers.set_flags(flags::PF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 2);
    cpu.registers.set_flags(0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 4);
}

#[test]
fn loops() {
    let (mut cpu, mut board) = boot(&[0xE2, 0xFE, 0xF4]);
    cpu.registers.set_cx(3);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cx(), 2);
    assert_eq!(cpu.registers.ip(), 0);
    cpu.step(&mut board);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cx(), 0);
    assert_eq!(cpu.registers.ip(), 2);

    let (mut cpu, mut board) = boot(&[0xE0, 0x00, 0xE1, 0x00, 0xF4]);
    cpu.registers.set_cx(2);
    cpu.registers.set_flags(flags::ZF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 2);
    assert_eq!(cpu.registers.cx(), 1);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 4);

    let (mut cpu, mut board) = boot(&[0x66, 0xE3, 0x02, 0x90, 0x90, 0xF4]);
    cpu.registers.set_ecx(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 5);
}

#[test]
fn call_ret_jmp() {
    let (mut cpu, mut board) = boot(&[0xE8, 0x02, 0x00, 0xF4, 0x90, 0xC3]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 5, "sp {:04x} mem {:04x}", cpu.registers.sp(), board.read_mem_u16(0x0FFE));
    assert_eq!(board.read_mem_u16(0x0FFE), 0x0003, "pushed");
    assert_eq!(cpu.registers.sp(), 0x0FFE, "sp after call");
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 3, "ret ip");
    assert_eq!(cpu.registers.sp(), 0x1000);

    let (mut cpu, mut board) = boot(&[0xE9, 0x01, 0x00, 0x90, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 4);

    let (mut cpu, mut board) = boot(&[0x66, 0xE9, 0x01, 0x00, 0x00, 0x00, 0x90, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 7);

    let (mut cpu, mut board) = boot(&[0xEB, 0x00, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 2);

    let (mut cpu, mut board) = boot(&[0xEA, 0x00, 0x20, 0x00, 0x10]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cs(), 0x1000);
    assert_eq!(cpu.registers.ip(), 0x2000);
}

#[test]
fn call_far_retf_enter_leave_bound() {
    let (mut cpu, mut board) = boot(&[0x9A, 0x10, 0x00, 0x00, 0x20, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.cs(), 0x2000);
    assert_eq!(cpu.registers.ip(), 0x0010);
    assert_eq!(board.read_mem_u16(0x0FFE), 0x0000);
    assert_eq!(board.read_mem_u16(0x0FFC), 0x0005);

    let (mut cpu, mut board) = boot(&[0xCB]);
    cpu.registers.set_sp(0x0FFC);
    board.write_mem_u16(0x0FFC, 0x1234);
    board.write_mem_u16(0x0FFE, 0x5678);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x1234);
    assert_eq!(cpu.registers.cs(), 0x5678);

    let (mut cpu, mut board) = boot(&[0xC8, 0x04, 0x00, 0x00, 0xF4]);
    cpu.registers.set_bp(0x2000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bp(), 0x0FFE);
    assert_eq!(cpu.registers.sp(), 0x0FFA);
    assert_eq!(board.read_mem_u16(0x0FFE), 0x2000);

    let (mut cpu, mut board) = boot(&[0xC9, 0xF4]);
    cpu.registers.set_bp(0x0FFE);
    cpu.registers.set_sp(0x0FFA);
    board.write_mem_u16(0x0FFE, 0x2000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bp(), 0x2000);
    assert_eq!(cpu.registers.sp(), 0x1000);

    let (mut cpu, mut board) = boot(&[]);
    board.write_mem_u8(0x200, 0x62);
    board.write_mem_u8(0x201, 0x06);
    board.write_mem_u8(0x202, 0x00);
    board.write_mem_u8(0x203, 0x30);
    board.write_mem_u16(0x3000, 0);
    board.write_mem_u16(0x3002, 10);
    board.write_mem_u16(0x0014, 0x4000);
    board.write_mem_u16(0x0016, 0x0000);
    cpu.registers.set_ip(0x200);
    cpu.registers.set_ax(100);
    cpu.registers.set_ds(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x4000);
    assert!(!flags::test_if(cpu.registers.flags()));
}

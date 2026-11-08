// Ver: 1 File: crate/x86/tests/extended_test.rs
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
fn lgdt_lidt_sgdt_smsw_live_on_cpu() {
    let (mut cpu, mut board) = boot(&[0x0F, 0x01, 0x16, 0x00, 0x20, 0xF4]); // LGDT [0x2000]
    board.write_mem_u16(0x2000, 0x1234);
    board.write_mem_u32(0x2002, 0x00AB_CDEF);
    cpu.step(&mut board);
    assert_eq!(cpu.gdtr_limit, 0x1234);
    assert_eq!(cpu.gdtr_base, 0x00AB_CDEF);
    assert!(!cpu.is_halted());

    let (mut cpu, mut board) = boot(&[0x0F, 0x01, 0x1E, 0x00, 0x30, 0xF4]); // LIDT [0x3000]
    board.write_mem_u16(0x3000, 0x0100);
    board.write_mem_u32(0x3002, 0x0000_4000);
    cpu.step(&mut board);
    assert_eq!(cpu.idtr_limit, 0x0100);
    assert_eq!(cpu.idtr_base, 0x4000);

    let (mut cpu, mut board) = boot(&[0x0F, 0x01, 0x06, 0x00, 0x40, 0xF4]); // SGDT [0x4000]
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(0x4000), 0xFFFF);
    assert_eq!(board.read_mem_u32(0x4002), 0);

    let (mut cpu, mut board) = boot(&[0x0F, 0x01, 0xE0, 0xF4]); // SMSW AX
    cpu.cr0 = 0x0000_0011;
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x0011);
}

#[test]
fn mov_cr0_and_movzx_and_pop_fs() {
    let (mut cpu, mut board) = boot(&[0x0F, 0x22, 0xC0, 0x0F, 0x20, 0xD8, 0xF4]);
    cpu.registers.set_eax(0x8000_0001);
    cpu.step(&mut board);
    assert_eq!(cpu.cr0, 0x8000_0001);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0x8000_0001);

    let (mut cpu, mut board) = boot(&[0x0F, 0xB7, 0xC3, 0xF4]); // MOVZX AX, BL
    cpu.registers.set_bx(0x0080);
    cpu.registers.set_ax(0xFFFF);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x0080);

    let (mut cpu, mut board) = boot(&[0x66, 0x0F, 0xB7, 0xC3, 0xF4]); // MOVZX EAX, BX
    cpu.registers.set_bx(0xABCD);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0x0000_ABCD);

    let (mut cpu, mut board) = boot(&[0x0F, 0xA1, 0xF4]);
    board.write_mem_u16(0x1000, 0x1357);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.fs(), 0x1357);
    assert_eq!(cpu.registers.sp(), 0x1002);
}

#[test]
fn jcc_near_and_unknown_0f_halts() {
    let (mut cpu, mut board) = boot(&[0x0F, 0x84, 0x02, 0x00, 0x90, 0x90, 0xF4]);
    cpu.registers.set_flags(flags::ZF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 6);

    let (mut cpu, mut board) = boot(&[0x0F, 0x82, 0x00, 0x00, 0xF4]);
    cpu.registers.set_flags(0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 4);
    assert!(!cpu.is_halted());

    let (mut cpu, mut board) = boot(&[0x0F, 0x00]);
    cpu.step(&mut board);
    assert!(cpu.is_halted());
}

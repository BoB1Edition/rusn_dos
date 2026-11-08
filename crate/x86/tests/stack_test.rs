// Ver: 1 File: crate/x86/tests/stack_test.rs
use bus::{Cpu, Machine, Motherboard};
use x86::cpu::X86Cpu;
use x86::flags;

fn boot(bytes: &[u8]) -> (X86Cpu, Motherboard) {
    let mut board = Motherboard::new(1024 * 1024);
    for (i, b) in bytes.iter().enumerate() {
        board.write_mem_u8(i as u32, *b);
    }
    let mut cpu = X86Cpu::new();
    cpu.registers.set_cs(0);
    cpu.registers.set_ip(0);
    cpu.registers.set_ss(0x1000);
    cpu.registers.set_sp(0x0100);
    (cpu, board)
}

fn stack_addr(sp: u16) -> u32 {
    (0x1000u32 << 4) + sp as u32
}

#[test]
fn push_pop_ax_roundtrip() {
    let (mut cpu, mut board) = boot(&[0x50, 0x58, 0xF4]);
    cpu.registers.set_ax(0xBEEF);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.sp(), 0x00FE);
    assert_eq!(board.read_mem_u16(stack_addr(0x00FE)), 0xBEEF);
    cpu.registers.set_ax(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0xBEEF);
    assert_eq!(cpu.registers.sp(), 0x0100);
}

#[test]
fn push_sp_is_value_before_decrement() {
    let (mut cpu, mut board) = boot(&[0x54, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.sp(), 0x00FE);
    assert_eq!(board.read_mem_u16(stack_addr(0x00FE)), 0x0100);
}

#[test]
fn push_eax_uses_esp_low16() {
    let (mut cpu, mut board) = boot(&[0x66, 0x50, 0xF4]);
    cpu.registers.set_eax(0x1122_3344);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.sp(), 0x00FC);
    assert_eq!(board.read_mem_u32(stack_addr(0x00FC)), 0x1122_3344);
}

#[test]
fn segment_push_pop_and_pop_ss_inhibits() {
    let (mut cpu, mut board) = boot(&[0x06, 0x07, 0x16, 0x17, 0x1E, 0x1F, 0x0E, 0xF4]);
    cpu.registers.set_es(0x1111);
    cpu.registers.set_ss(0x1000);
    cpu.registers.set_ds(0x3333);
    cpu.registers.set_cs(0);
    cpu.step(&mut board);
    cpu.registers.set_es(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.es(), 0x1111);

    cpu.step(&mut board);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ss(), 0x1000);
    assert!(cpu.inhibit_interrupts);

    cpu.step(&mut board);
    cpu.registers.set_ds(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ds(), 0x3333);

    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(stack_addr(cpu.registers.sp())), 0x0000);
}

#[test]
fn push_imm_and_sign_extend() {
    let (mut cpu, mut board) = boot(&[0x68, 0x34, 0x12, 0x6A, 0x80, 0xF4]);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(stack_addr(cpu.registers.sp())), 0x1234);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(stack_addr(cpu.registers.sp())), 0xFF80);

    let (mut cpu, mut board) = boot(&[0x66, 0x6A, 0xFF, 0xF4]);
    cpu.registers.set_ss(0x1000);
    cpu.registers.set_sp(0x0100);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u32(stack_addr(cpu.registers.sp())), 0xFFFF_FFFF);
}

#[test]
fn pop_rm_reg_and_mem() {
    let (mut cpu, mut board) = boot(&[0x8F, 0xC3, 0xF4]); // POP BX
    board.write_mem_u16(stack_addr(0x0100), 0xABCD);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bx(), 0xABCD);
    assert_eq!(cpu.registers.sp(), 0x0102);

    let (mut cpu, mut board) = boot(&[0x8F, 0x06, 0x00, 0x40, 0xF4]); // POP [0x4000]
    cpu.registers.set_ss(0x1000);
    cpu.registers.set_sp(0x0100);
    cpu.registers.set_ds(0);
    board.write_mem_u16(stack_addr(0x0100), 0x55AA);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(0x4000), 0x55AA);

    let (mut cpu, mut board) = boot(&[0x8F, 0xC8, 0xF4]); // /1 invalid
    cpu.registers.set_ss(0x1000);
    cpu.registers.set_sp(0x0100);
    cpu.step(&mut board);
    assert!(cpu.is_halted());
}

#[test]
fn pusha_popa_skip_sp() {
    let (mut cpu, mut board) = boot(&[0x60, 0x61, 0xF4]);
    cpu.registers.set_ax(0x0001);
    cpu.registers.set_cx(0x0002);
    cpu.registers.set_dx(0x0003);
    cpu.registers.set_bx(0x0004);
    cpu.registers.set_bp(0x0005);
    cpu.registers.set_si(0x0006);
    cpu.registers.set_di(0x0007);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.sp(), 0x0100 - 16);
    assert_eq!(board.read_mem_u16(stack_addr(0x00FE)), 0x0001);
    assert_eq!(board.read_mem_u16(stack_addr(0x00F6)), 0x0100);
    cpu.registers.set_ax(0);
    cpu.registers.set_di(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x0001);
    assert_eq!(cpu.registers.di(), 0x0007);
    assert_eq!(cpu.registers.sp(), 0x0100);
}

#[test]
fn pushf_popf() {
    let (mut cpu, mut board) = boot(&[0x9C, 0x9D, 0xF4]);
    cpu.registers.set_flags(flags::CF | flags::ZF | flags::IF | 0x2);
    cpu.step(&mut board);
    assert_eq!(
        board.read_mem_u16(stack_addr(cpu.registers.sp())) & (flags::CF | flags::ZF | flags::IF),
        flags::CF | flags::ZF | flags::IF
    );
    board.write_mem_u16(stack_addr(cpu.registers.sp()), flags::SF | 0x2);
    cpu.step(&mut board);
    assert!(flags::test_sf(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));
    assert_eq!(cpu.registers.flags() & 0x2, 0x2);
}

#[test]
fn xchg_forms() {
    let (mut cpu, mut board) = boot(&[0x91, 0x86, 0xE3, 0x87, 0x06, 0x00, 0x20, 0xF4]);
    cpu.registers.set_ax(0x1111);
    cpu.registers.set_bx(0x0022);
    cpu.registers.set_cx(0x2222);
    cpu.registers.set_ds(0);
    board.write_mem_u16(0x2000, 0xABCD);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x2222);
    assert_eq!(cpu.registers.cx(), 0x1111);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ah(), 0x22);
    assert_eq!(cpu.registers.bl(), 0x22);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0xABCD);
    assert_eq!(board.read_mem_u16(0x2000), 0x2222);

    let (mut cpu, mut board) = boot(&[0x66, 0x93, 0xF4]);
    cpu.registers.set_eax(0x1111_1111);
    cpu.registers.set_ebx(0x2222_2222);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0x2222_2222);
    assert_eq!(cpu.registers.ebx(), 0x1111_1111);
}

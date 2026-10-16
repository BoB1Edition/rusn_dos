// Ver: 1 File: crate/x86/tests/group_test.rs
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
fn group80_ops_reg_and_mem() {
    let (mut cpu, mut board) = boot(&[0x80, 0xC0, 0x01, 0xF4]); // ADD AL, 1
    cpu.registers.set_al(0xFF);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0);
    assert!(flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0x82, 0xC8, 0x0F, 0xF4]); // OR AL, 0x0F via 82 alias
    cpu.registers.set_al(0xF0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0xFF);

    let (mut cpu, mut board) = boot(&[0x80, 0x2E, 0x00, 0x20, 0x01, 0xF4]); // SUB [0x2000], 1
    board.write_mem_u8(0x2000, 0x00);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u8(0x2000), 0xFF);
    assert!(flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0x80, 0xF8, 0x05, 0xF4]); // CMP AL, 5
    cpu.registers.set_al(0x05);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x05);
    assert!(flags::test_zf(cpu.registers.flags()));
}

#[test]
fn group81_83_sign_extend() {
    let (mut cpu, mut board) = boot(&[0x81, 0xC3, 0x00, 0x01, 0xF4]); // ADD BX, 0x100
    cpu.registers.set_bx(1);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bx(), 0x101);

    let (mut cpu, mut board) = boot(&[0x83, 0xE8, 0x02, 0xF4]); // SUB AX, 2
    cpu.registers.set_ax(5);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 3);

    let (mut cpu, mut board) = boot(&[0x66, 0x83, 0xC0, 0xFF, 0xF4]); // ADD EAX, -1
    cpu.registers.set_eax(1);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.eax(), 0);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_zf(cpu.registers.flags()));
}

#[test]
fn shifts_rol_shl_sar_rcl() {
    let (mut cpu, mut board) = boot(&[0xD0, 0xC0, 0xF4]); // ROL AL, 1
    cpu.registers.set_al(0x80);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x01);
    assert!(flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xC0, 0xE0, 0x04, 0xF4]); // SHL AL, 4
    cpu.registers.set_al(0x01);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x10);

    let (mut cpu, mut board) = boot(&[0xD0, 0xF8, 0xF4]); // SAR AL, 1
    cpu.registers.set_al(0x80);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0xC0);
    assert!(!flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xD0, 0xD0, 0xF4]); // RCL AL, 1
    cpu.registers.set_al(0x80);
    cpu.registers.set_flags(flags::CF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x01);
    assert!(flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xD2, 0xE8, 0xF4]); // SHR AL, CL
    cpu.registers.set_al(0x80);
    cpu.registers.set_cl(3);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x10);

    let (mut cpu, mut board) = boot(&[0xD1, 0xE3, 0xF4]); // SHL BX, 1
    cpu.registers.set_bx(0x4000);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.bx(), 0x8000);
    assert!(flags::test_of(cpu.registers.flags()));
}

#[test]
fn f6_test_not_neg_mul_div() {
    let (mut cpu, mut board) = boot(&[0xF6, 0xC0, 0x0F, 0xF4]); // TEST AL, 0x0F
    cpu.registers.set_al(0xF0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0xF0);
    assert!(flags::test_zf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xF6, 0xD0, 0xF4]); // NOT AL
    cpu.registers.set_al(0x0F);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0xF0);

    let (mut cpu, mut board) = boot(&[0xF6, 0xD8, 0xF4]); // NEG AL
    cpu.registers.set_al(0x01);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0xFF);
    assert!(flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xF6, 0xE3, 0xF4]); // MUL BL
    cpu.registers.set_al(0x10);
    cpu.registers.set_bl(0x10);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ax(), 0x0100);
    assert!(flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xF6, 0xF3, 0xF4]); // DIV BL
    cpu.registers.set_ax(0x0100);
    cpu.registers.set_bl(0x10);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x10);
    assert_eq!(cpu.registers.ah(), 0x00);

    let (mut cpu, mut board) = boot(&[0x90]);
    board.write_mem_u8(0x300, 0xF6);
    board.write_mem_u8(0x301, 0xF0);
    board.write_mem_u16(0, 0x0500);
    board.write_mem_u16(2, 0);
    cpu.registers.set_ip(0x300);
    cpu.registers.set_al(0);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x0500);
}

#[test]
fn fe_ff_inc_call_push() {
    let (mut cpu, mut board) = boot(&[0xFE, 0xC0, 0xF4]); // INC AL
    cpu.registers.set_al(0x7F);
    cpu.registers.set_flags(flags::CF | 0x2);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.al(), 0x80);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_of(cpu.registers.flags()));

    let (mut cpu, mut board) = boot(&[0xFE, 0xD0, 0xF4]); // /2 undefined
    cpu.step(&mut board);
    assert!(cpu.is_halted());

    let (mut cpu, mut board) = boot(&[0xFF, 0xD0, 0xF4]); // CALL AX
    cpu.registers.set_ax(0x0100);
    cpu.step(&mut board);
    assert_eq!(cpu.registers.ip(), 0x0100);
    assert_eq!(board.read_mem_u16(0x0FFE), 2);

    let (mut cpu, mut board) = boot(&[0xFF, 0xF0, 0xF4]); // PUSH AX
    cpu.registers.set_ax(0xABCD);
    cpu.step(&mut board);
    assert_eq!(board.read_mem_u16(0x0FFE), 0xABCD);

    let (mut cpu, mut board) = boot(&[0xFF, 0xF8, 0xF4]); // /7
    cpu.step(&mut board);
    assert!(cpu.is_halted());
}

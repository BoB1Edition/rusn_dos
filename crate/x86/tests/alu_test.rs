// Ver: 1 File: crate/x86/tests/alu_test.rs
use bus::{Cpu, Machine, Motherboard};
use x86::cpu::X86Cpu;
use x86::flags;

fn cpu_at(bytes: &[u8]) -> (X86Cpu, Motherboard) {
    let mut board = Motherboard::new(1024 * 1024);
    for (i, b) in bytes.iter().enumerate() {
        board.write_mem_u8(i as u32, *b);
    }
    let mut cpu = X86Cpu::new();
    cpu.registers.set_cs(0);
    cpu.registers.set_ip(0);
    cpu.registers.set_ds(0);
    (cpu, board)
}

fn step(cpu: &mut X86Cpu, board: &mut Motherboard) {
    cpu.step(board);
}

fn flags_of(cpu: &X86Cpu) -> (bool, bool, bool, bool, bool, bool) {
    let f = cpu.registers.flags();
    (
        flags::test_cf(f),
        flags::test_of(f),
        flags::test_sf(f),
        flags::test_zf(f),
        flags::test_af(f),
        flags::test_pf(f),
    )
}

#[test]
fn add_al_imm8_flags() {
    let (mut cpu, mut board) = cpu_at(&[0x04, 0x01, 0xF4]);
    cpu.registers.set_al(0xFF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x00);
    assert_eq!(flags_of(&cpu), (true, false, false, true, true, true));
    assert_eq!(cpu.registers.ip(), 2);
}

#[test]
fn add_ax_imm16_and_66() {
    let (mut cpu, mut board) = cpu_at(&[0x05, 0x00, 0x01, 0xF4]);
    cpu.registers.set_ax(0x00FF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x01FF);
    assert_eq!(flags_of(&cpu).0, false);

    let (mut cpu, mut board) = cpu_at(&[0x66, 0x05, 0x01, 0x00, 0x00, 0x00, 0xF4]);
    cpu.registers.set_eax(0xFFFF_FFFF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.eax(), 0);
    assert_eq!(flags_of(&cpu), (true, false, false, true, true, true));
}

#[test]
fn add_rm8_r8_reg_and_mem() {
    let (mut cpu, mut board) = cpu_at(&[0x00, 0xC8, 0xF4]); // ADD AL, CL
    cpu.registers.set_al(0x10);
    cpu.registers.set_cl(0x20);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x30);
    assert_eq!(flags_of(&cpu).0, false);

    let (mut cpu, mut board) = cpu_at(&[0x00, 0x06, 0x00, 0x20, 0xF4]); // ADD [0x2000], AL
    cpu.registers.set_al(0x05);
    board.write_mem_u8(0x2000, 0xFE);
    step(&mut cpu, &mut board);
    assert_eq!(board.read_mem_u8(0x2000), 0x03);
    assert_eq!(flags_of(&cpu), (true, false, false, false, true, true));
}

#[test]
fn add_r16_rm16_and_overflow() {
    let (mut cpu, mut board) = cpu_at(&[0x03, 0xC3, 0xF4]); // ADD AX, BX
    cpu.registers.set_ax(0x7FFF);
    cpu.registers.set_bx(0x0001);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x8000);
    assert_eq!(flags_of(&cpu), (false, true, true, false, true, true));
}

#[test]
fn or_and_xor_imm_clear_cf_of() {
    let (mut cpu, mut board) = cpu_at(&[0x0C, 0x0F, 0xF4]);
    cpu.registers.set_al(0xF0);
    cpu.registers.set_flags(flags::CF | flags::OF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0xFF);
    assert!(!flags::test_cf(cpu.registers.flags()));
    assert!(!flags::test_of(cpu.registers.flags()));
    assert!(flags::test_sf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x24, 0x0F, 0xF4]);
    cpu.registers.set_al(0xFF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x0F);
    assert!(flags::test_pf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x34, 0xFF, 0xF4]);
    cpu.registers.set_al(0xFF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x00);
    assert!(flags::test_zf(cpu.registers.flags()));
    assert!(flags::test_pf(cpu.registers.flags()));
}

#[test]
fn and_ax_imm16_hole() {
    let (mut cpu, mut board) = cpu_at(&[0x25, 0x0F, 0x00, 0xF4]);
    cpu.registers.set_ax(0xFFFF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x000F);
    assert!(!flags::test_zf(cpu.registers.flags()));
}

#[test]
fn xor_ax_imm16_and_eax() {
    let (mut cpu, mut board) = cpu_at(&[0x35, 0xFF, 0x00, 0xF4]);
    cpu.registers.set_ax(0x00FF);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x0000);
    assert!(flags::test_zf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x66, 0x35, 0x00, 0x00, 0x00, 0xFF, 0xF4]);
    cpu.registers.set_eax(0xFF00_0000);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.eax(), 0);
    assert!(flags::test_zf(cpu.registers.flags()));
}

#[test]
fn adc_uses_carry_including_ax_hole() {
    let (mut cpu, mut board) = cpu_at(&[0x14, 0x01, 0xF4]);
    cpu.registers.set_al(0x01);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x03);
    assert!(!flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x15, 0x00, 0x00, 0xF4]);
    cpu.registers.set_ax(0xFFFF);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x0000);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_zf(cpu.registers.flags()));
}

#[test]
fn sbb_holes_and_borrow() {
    let (mut cpu, mut board) = cpu_at(&[0x1C, 0x01, 0xF4]);
    cpu.registers.set_al(0x00);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0xFE);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_sf(cpu.registers.flags()));
    assert!(flags::test_af(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x1A, 0xD8, 0xF4]); // SBB BL, AL
    cpu.registers.set_al(0x01);
    cpu.registers.set_bl(0x10);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.bl(), 0x0F);
    assert!(flags::test_af(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x1D, 0x01, 0x00, 0xF4]);
    cpu.registers.set_ax(0x0000);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0xFFFE);
    assert!(flags::test_cf(cpu.registers.flags()));
}

#[test]
fn sub_al_and_ax_imm_hole() {
    let (mut cpu, mut board) = cpu_at(&[0x2C, 0x01, 0xF4]);
    cpu.registers.set_al(0x00);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0xFF);
    assert_eq!(flags_of(&cpu), (true, false, true, false, true, true));

    let (mut cpu, mut board) = cpu_at(&[0x2D, 0x01, 0x00, 0xF4]);
    cpu.registers.set_ax(0x8000);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x7FFF);
    assert!(flags::test_of(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));
}

#[test]
fn sub_rm8_r8_was_missing() {
    let (mut cpu, mut board) = cpu_at(&[0x28, 0xC1, 0xF4]); // SUB CL, AL
    cpu.registers.set_al(0x05);
    cpu.registers.set_cl(0x03);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.cl(), 0xFE);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_sf(cpu.registers.flags()));
}

#[test]
fn cmp_does_not_write() {
    let (mut cpu, mut board) = cpu_at(&[0x3C, 0x42, 0xF4]);
    cpu.registers.set_al(0x42);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x42);
    assert!(flags::test_zf(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x3D, 0x01, 0x00, 0xF4]);
    cpu.registers.set_ax(0x0000);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x0000);
    assert!(flags::test_cf(cpu.registers.flags()));
}

#[test]
fn cmp_rm16_mem() {
    let (mut cpu, mut board) = cpu_at(&[0x39, 0x06, 0x10, 0x00, 0xF4]); // CMP [0x0010], AX
    cpu.registers.set_ax(0x0002);
    board.write_mem_u16(0x0010, 0x0005);
    step(&mut cpu, &mut board);
    assert_eq!(board.read_mem_u16(0x0010), 0x0005);
    assert!(!flags::test_zf(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));
}

#[test]
fn inc_dec_preserve_cf() {
    let (mut cpu, mut board) = cpu_at(&[0x40, 0xF4]);
    cpu.registers.set_ax(0xFFFF);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x0000);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_zf(cpu.registers.flags()));
    assert!(flags::test_af(cpu.registers.flags()));
    assert!(!flags::test_of(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x66, 0x40, 0xF4]);
    cpu.registers.set_eax(0x7FFF_FFFF);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.eax(), 0x8000_0000);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_of(cpu.registers.flags()));
    assert!(flags::test_sf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x48, 0xF4]);
    cpu.registers.set_ax(0x8000);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0x7FFF);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_of(cpu.registers.flags()));
    assert!(!flags::test_sf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x49, 0xF4]);
    cpu.registers.set_cx(0x0001);
    cpu.registers.set_flags(flags::CF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.cx(), 0);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_zf(cpu.registers.flags()));
}

#[test]
fn logical_rm_forms_and_32() {
    let (mut cpu, mut board) = cpu_at(&[0x08, 0xD8, 0xF4]); // OR AL, BL
    cpu.registers.set_al(0x0F);
    cpu.registers.set_bl(0xF0);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0xFF);

    let (mut cpu, mut board) = cpu_at(&[0x66, 0x21, 0xC3, 0xF4]); // AND EBX, EAX
    cpu.registers.set_eax(0x0000_00FF);
    cpu.registers.set_ebx(0xFFFF_000F);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ebx(), 0x0000_000F);
    assert!(!flags::test_zf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x33, 0xC0, 0xF4]); // XOR AX, AX
    cpu.registers.set_ax(0x1234);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.ax(), 0);
    assert!(flags::test_zf(cpu.registers.flags()));
    assert!(flags::test_pf(cpu.registers.flags()));
}

#[test]
fn daa_and_das() {
    let (mut cpu, mut board) = cpu_at(&[0x27, 0xF4]);
    cpu.registers.set_al(0x7A);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x80);
    assert!(flags::test_af(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_sf(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x27, 0xF4]);
    cpu.registers.set_al(0x9A);
    cpu.registers.set_flags(flags::AF | flags::OF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x00);
    assert!(flags::test_cf(cpu.registers.flags()));
    assert!(flags::test_zf(cpu.registers.flags()));
    assert!(flags::test_af(cpu.registers.flags()));
    assert!(flags::test_of(cpu.registers.flags()));

    let (mut cpu, mut board) = cpu_at(&[0x2F, 0xF4]);
    cpu.registers.set_al(0x35);
    cpu.registers.set_flags(flags::AF | 0x2);
    step(&mut cpu, &mut board);
    assert_eq!(cpu.registers.al(), 0x2F);
    assert!(flags::test_af(cpu.registers.flags()));
    assert!(!flags::test_cf(cpu.registers.flags()));
}

#[test]
fn unknown_opcode_halts() {
    let (mut cpu, mut board) = cpu_at(&[0x0F, 0x00]);
    step(&mut cpu, &mut board);
    assert!(cpu.is_halted());
}

use crate::cpu::X86Cpu;
use crate::flags;

pub fn set_flags(cpu: &mut X86Cpu, flags: u16) {
    cpu.registers.set_flags(flags);
}

pub fn add8(dst: u8, src: u8) -> (u8, bool, bool, bool) {
    let res = dst as u16 + src as u16;
    let result = res as u8;
    let cf = res > 0xFF;
    let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
    let of = ((dst ^ src) & 0x80) == 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn add16(dst: u16, src: u16) -> (u16, bool, bool, bool) {
    let res = dst as u32 + src as u32;
    let result = res as u16;
    let cf = res > 0xFFFF;
    let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
    let of = ((dst ^ src) & 0x8000) == 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn add32(dst: u32, src: u32) -> (u32, bool, bool, bool) {
    let res = dst as u64 + src as u64;
    let result = res as u32;
    let cf = res > 0xFFFF_FFFF;
    let af = ((dst & 0x0F) + (src & 0x0F)) > 0x0F;
    let of = ((dst ^ src) & 0x8000_0000) == 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn adc8(dst: u8, src: u8, carry: bool) -> (u8, bool, bool, bool) {
    let c = if carry { 1u16 } else { 0 };
    let res = dst as u16 + src as u16 + c;
    let result = res as u8;
    let cf = res > 0xFF;
    let af = ((dst & 0x0F) as u16 + (src & 0x0F) as u16 + c) > 0x0F;
    let of = ((dst ^ src) & 0x80) == 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn adc16(dst: u16, src: u16, carry: bool) -> (u16, bool, bool, bool) {
    let c = if carry { 1u32 } else { 0 };
    let res = dst as u32 + src as u32 + c;
    let result = res as u16;
    let cf = res > 0xFFFF;
    let af = ((dst & 0x0F) as u32 + (src & 0x0F) as u32 + c) > 0x0F;
    let of = ((dst ^ src) & 0x8000) == 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn adc32(dst: u32, src: u32, carry: bool) -> (u32, bool, bool, bool) {
    let c = if carry { 1u64 } else { 0 };
    let res = dst as u64 + src as u64 + c;
    let result = res as u32;
    let cf = res > 0xFFFF_FFFF;
    let af = ((dst & 0x0F) as u64 + (src & 0x0F) as u64 + c) > 0x0F;
    let of = ((dst ^ src) & 0x8000_0000) == 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn sub8(dst: u8, src: u8) -> (u8, bool, bool, bool) {
    let result = dst.wrapping_sub(src);
    let cf = dst < src;
    let af = (dst & 0x0F) < (src & 0x0F);
    let of = ((dst ^ src) & 0x80) != 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn sub16(dst: u16, src: u16) -> (u16, bool, bool, bool) {
    let result = dst.wrapping_sub(src);
    let cf = dst < src;
    let af = (dst & 0x0F) < (src & 0x0F);
    let of = ((dst ^ src) & 0x8000) != 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn sub32(dst: u32, src: u32) -> (u32, bool, bool, bool) {
    let result = dst.wrapping_sub(src);
    let cf = dst < src;
    let af = (dst & 0x0F) < (src & 0x0F);
    let of = ((dst ^ src) & 0x8000_0000) != 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn sbb8(dst: u8, src: u8, carry: bool) -> (u8, bool, bool, bool) {
    let c = if carry { 1u16 } else { 0 };
    let total = src as u16 + c;
    let result = (dst as u16).wrapping_sub(total) as u8;
    let cf = (dst as u16) < total;
    let af = ((dst ^ src ^ result) & 0x10) != 0;
    let of = ((dst ^ src) & 0x80) != 0 && ((dst ^ result) & 0x80) != 0;
    (result, cf, of, af)
}

pub fn sbb16(dst: u16, src: u16, carry: bool) -> (u16, bool, bool, bool) {
    let c = if carry { 1u32 } else { 0 };
    let total = src as u32 + c;
    let result = (dst as u32).wrapping_sub(total) as u16;
    let cf = (dst as u32) < total;
    let af = ((dst ^ src ^ result) & 0x10) != 0;
    let of = ((dst ^ src) & 0x8000) != 0 && ((dst ^ result) & 0x8000) != 0;
    (result, cf, of, af)
}

pub fn sbb32(dst: u32, src: u32, carry: bool) -> (u32, bool, bool, bool) {
    let c = if carry { 1u64 } else { 0 };
    let total = src as u64 + c;
    let result = (dst as u64).wrapping_sub(total) as u32;
    let cf = (dst as u64) < total;
    let af = ((dst ^ src ^ result) & 0x10) != 0;
    let of = ((dst ^ src) & 0x8000_0000) != 0 && ((dst ^ result) & 0x8000_0000) != 0;
    (result, cf, of, af)
}

pub fn apply_arith8(cpu: &mut X86Cpu, result: u8, cf: bool, of: bool, af: bool) {
    let flags = flags::compute_flags_u8(cpu.registers.flags(), result, cf, of, af);
    set_flags(cpu, flags);
}

pub fn apply_arith16(cpu: &mut X86Cpu, result: u16, cf: bool, of: bool, af: bool) {
    let flags = flags::compute_flags_u16(cpu.registers.flags(), result, cf, of, af);
    set_flags(cpu, flags);
}

pub fn apply_arith32(cpu: &mut X86Cpu, result: u32, cf: bool, of: bool, af: bool) {
    let flags = flags::compute_flags_u32(cpu.registers.flags(), result, cf, of, af);
    set_flags(cpu, flags);
}

pub fn apply_logic8(cpu: &mut X86Cpu, result: u8) {
    set_flags(cpu, flags::compute_logical_flags_u8(cpu.registers.flags(), result));
}

pub fn apply_logic16(cpu: &mut X86Cpu, result: u16) {
    set_flags(cpu, flags::compute_logical_flags_u16(cpu.registers.flags(), result));
}

pub fn apply_logic32(cpu: &mut X86Cpu, result: u32) {
    set_flags(cpu, flags::compute_logical_flags_u32(cpu.registers.flags(), result));
}

pub fn carry(cpu: &X86Cpu) -> bool {
    flags::test_cf(cpu.registers.flags())
}

pub enum Width {
    B8,
    B16,
    B32,
}

pub enum Op {
    Add,
    Or,
    Adc,
    Sbb,
    And,
    Sub,
    Xor,
    Cmp,
    Test,
}

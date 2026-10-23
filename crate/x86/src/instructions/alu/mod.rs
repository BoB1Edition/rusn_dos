mod core;
mod util;
mod eval;
mod inst;
mod bcd;
mod incdec;
mod group;

pub use core::{add8, add16, add32, adc8, adc16, adc32, sub8, sub16, sub32, sbb8, sbb16, sbb32};
pub use inst::{
    test_rm8_r8, test_rm16_r16, test_rm32_r32,
    test_al_imm8, test_ax_imm16, test_eax_imm32,
    add_rm8_r8, add_rm16_r16, add_rm32_r32,
    add_r8_rm8, add_r16_rm16, add_r32_rm32,
    add_al_imm8, add_ax_imm16, add_eax_imm32,
    or_rm8_r8, or_rm16_r16, or_rm32_r32,
    or_r8_rm8, or_r16_rm16, or_r32_rm32,
    or_al_imm8, or_ax_imm16, or_eax_imm32,
    adc_rm8_r8, adc_rm16_r16, adc_rm32_r32,
    adc_r8_rm8, adc_r16_rm16, adc_r32_rm32,
    adc_al_imm8, adc_ax_imm16, adc_eax_imm32,
    sbb_rm8_r8, sbb_rm16_r16, sbb_rm32_r32,
    sbb_r8_rm8, sbb_r16_rm16, sbb_r32_rm32,
    sbb_al_imm8, sbb_ax_imm16, sbb_eax_imm32,
    and_rm8_r8, and_rm16_r16, and_rm32_r32,
    and_r8_rm8, and_r16_rm16, and_r32_rm32,
    and_al_imm8, and_ax_imm16, and_eax_imm32,
    sub_rm8_r8, sub_rm16_r16, sub_rm32_r32,
    sub_r8_rm8, sub_r16_rm16, sub_r32_rm32,
    sub_al_imm8, sub_ax_imm16, sub_eax_imm32,
    xor_rm8_r8, xor_rm16_r16, xor_rm32_r32,
    xor_r8_rm8, xor_r16_rm16, xor_r32_rm32,
    xor_al_imm8, xor_ax_imm16, xor_eax_imm32,
    cmp_rm8_r8, cmp_rm16_r16, cmp_rm32_r32,
    cmp_r8_rm8, cmp_r16_rm16, cmp_r32_rm32,
    cmp_al_imm8, cmp_ax_imm16, cmp_eax_imm32,
};
pub use bcd::{daa, das};
pub use incdec::{inc_reg, dec_reg};
pub use group::{group80, group81, group83};

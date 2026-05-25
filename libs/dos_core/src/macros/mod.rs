// Ver: 4 File: ./libs/dos_core/src/instructions/macros/mod.rs

#[macro_export]
macro_rules! inc_reg16 {
    ($name:ident, $get:ident, $set:ident, $opcode:literal) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();

            let old = machine.registers.$get();
            let result = old.wrapping_add(1);
            let af = ((old & 0x0F) + 1) > 0x0F;
            let of = old == 0x7FFF;
            let current_cf = (machine.registers.flags() & $crate::flags::CF) != 0;

            machine.registers.$set(result);
            machine
                .registers
                .set_flags($crate::flags::compute_flags_u16(
                    machine.registers.flags(),
                    result,
                    current_cf,
                    of,
                    af,
                ));

            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! dec_reg16 {
    ($name:ident, $get:ident, $set:ident, $opcode:literal) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let mut bytes = prev.to_vec();
            bytes.push($opcode);

            let old = machine.registers.$get();
            let result = old.wrapping_sub(1);
            let af = (old & 0x0F) == 0;
            let of = old == 0x8000;
            let current_cf = (machine.registers.flags() & $crate::flags::CF) != 0;

            machine.registers.$set(result);
            machine
                .registers
                .set_flags($crate::flags::compute_flags_u16(
                    machine.registers.flags(),
                    result,
                    current_cf,
                    of,
                    af,
                ));

            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! mov_reg8_imm8 {
    ($name:ident, $set:ident, $opcode:literal) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let mut bytes = prev.to_vec();
            let imm = machine.read_instr_u8(machine.registers.ip());
            bytes.push(imm);
            machine.log_instruction(csip, &bytes).ok();
            machine.registers.$set(imm);
            machine.registers.step(None);
        }
    };
}

#[macro_export]
macro_rules! push_reg16 {
    ($name:ident, $get:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            machine
                .registers
                .set_sp(machine.registers.sp().wrapping_sub(2));
            machine.write_u16(
                machine.registers.ss(),
                machine.registers.sp(),
                machine.registers.$get(),
            );
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! pop_reg16 {
    ($name:ident, $set:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [
                machine.registers.cs(),
                machine.registers.ip() - prev.len() as u16,
            ];
            let bytes = prev.to_vec();
            let reg = machine.read_u16(machine.registers.ss(), machine.registers.sp());
            machine
                .registers
                .set_sp(machine.registers.sp().wrapping_add(2));
            machine.registers.$set(reg);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! xchg_ax_reg16 {
    ($name:ident, $get:ident, $set:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
            let bytes = prev.to_vec();
            let ax = machine.registers.ax();
            let other = machine.registers.$get();
            machine.registers.set_ax(other);
            machine.registers.$set(ax);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! xchg_eax_reg32 {
    ($name:ident, $get:ident, $set:ident) => {
        pub(crate) fn $name(machine: &mut $crate::DosMachine, prev: &[u8]) {
            let csip = [machine.registers.cs(), machine.registers.ip() - prev.len() as u16];
            let bytes = prev.to_vec();
            let eax = machine.registers.eax();
            let other = machine.registers.$get();
            machine.registers.set_eax(other);
            machine.registers.$set(eax);
            machine.log_instruction(csip, &bytes).ok();
        }
    };
}

#[macro_export]
macro_rules! dispatch_op32 {
    ($machine:expr, $op32:expr, $op16:expr) => {
        if $machine.has_operand_size_prefix {
            $op32
        } else {
            $op16
        }
    };
}
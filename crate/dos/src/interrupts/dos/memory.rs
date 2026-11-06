// Ver: 2 File: ./crate/dos/src/interrupts/dos/memory.rs

use crate::mcb::{self};
use crate::system::DosSystem;

use super::misc::set_carry_flag;

pub fn allocate_memory_handler(sys: &mut DosSystem) {
    let paragraphs = sys.cpu.registers.bx();
    let first = sys.first_mcb_segment;
    match mcb::allocate(&mut sys.machine, first, paragraphs) {
        Some(segment) => {
            let mcb_seg = segment.wrapping_sub(1);
            let mut mcb = mcb::MCB::read(&sys.machine, mcb_seg);
            mcb.owner = sys.cpu.registers.ds();
            mcb.write(&mut sys.machine, mcb_seg);

            sys.cpu.registers.set_ax(segment);
            set_carry_flag(sys, false);
        }
        None => {
            let max = mcb::max_available(&sys.machine, first);
            sys.cpu.registers.set_ax(0x08);
            sys.cpu.registers.set_bx(max);
            set_carry_flag(sys, true);
            mcb::dump_chain(&sys.machine, first);
        }
    }
}

pub fn free_memory_handler(sys: &mut DosSystem) {
    let segment = sys.cpu.registers.es();
    let first = sys.first_mcb_segment;
    match mcb::free(&mut sys.machine, first, segment) {
        Ok(()) => set_carry_flag(sys, false),
        Err(code) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(code);
            mcb::dump_chain(&sys.machine, first);
        }
    }
}

pub fn modify_memory_handler(sys: &mut DosSystem) {
    let segment = sys.cpu.registers.es();
    let new_paragraphs = sys.cpu.registers.bx();
    let first = sys.first_mcb_segment;
    match mcb::modify(&mut sys.machine, first, segment, new_paragraphs) {
        Ok(_) => set_carry_flag(sys, false),
        Err(max) => {
            set_carry_flag(sys, true);
            sys.cpu.registers.set_ax(0x08);
            sys.cpu.registers.set_bx(max);
            mcb::dump_chain(&sys.machine, first);
        }
    }
}


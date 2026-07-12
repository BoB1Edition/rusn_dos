// Ver: 1 File: crate/x86/tests/integration_test.rs
use bus::{Cpu, Machine, Motherboard};
use x86::cpu::X86Cpu;

#[test]
fn test_basic_cpu_execution() {
    // 1. Создаем "Материнскую плату" с 1 МБ памяти
    let mut motherboard = Motherboard::new(1024 * 1024);

    // 2. Создаем процессор x86
    let mut cpu = X86Cpu::new();
    cpu.registers.set_cs(0x0000);
    cpu.registers.set_ip(0x0000);

    // 3. Загружаем программу: MOV AL, 0x42 -> NOP -> HLT
    motherboard.write_mem_u8(0x00000, 0xB0); // MOV AL, imm8
    motherboard.write_mem_u8(0x00001, 0x42); // imm8 = 0x42
    motherboard.write_mem_u8(0x00002, 0x90); // NOP
    motherboard.write_mem_u8(0x00003, 0xF4); // HLT

    // 4. Запускаем выполнение
    let mut steps = 0;
    while !cpu.is_halted() && steps < 10 {
        cpu.step(&mut motherboard);
        steps += 1;
    }

    // 5. Проверяем результаты
    assert!(cpu.is_halted(), "CPU should have halted");
    assert_eq!(cpu.registers.al(), 0x42, "AL should be 0x42");
    assert_eq!(steps, 3, "Should take exactly 3 steps (MOV, NOP, HLT)");
}
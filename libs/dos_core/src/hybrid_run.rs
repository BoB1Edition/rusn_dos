pub fn run_hybrid(machine: &mut DosMachine, cpu: &mut X86Cpu) {
    while !machine.halted {
        // 1. Выполняем одну инструкцию через новый CPU
        cpu.step(machine); // machine реализует bus::Machine
        
        // 2. Обработка клавиатуры и видео (по оставляем старую логику из run.rs)
        // ...
        
        // 3. Обработка таймера и прерываний (по оставляем старую логику)
        // ...
    }
}
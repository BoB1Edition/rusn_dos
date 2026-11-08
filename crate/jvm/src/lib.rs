// Java Virtual Machine stub
// Запуск .class и .jar файлов
// Интеграция через native-интерфейс для вызовов API эмулятора
// Стековая архитектура, 256 опкодов

#![allow(dead_code)]

pub struct JavaVm {
    pub halted: bool,
}

impl JavaVm {
    pub fn new() -> Self {
        log::info!("JVM stub created");
        Self { halted: false }
    }

    pub fn step(&mut self) {
        // TODO: реализовать шаг эмуляции JVM
    }

    pub fn load_class(&mut self, _data: &[u8]) {
        log::info!("JVM class loading not yet implemented");
    }

    pub fn load_jar(&mut self, _data: &[u8]) {
        log::info!("JVM jar loading not yet implemented");
    }
}

impl Default for JavaVm {
    fn default() -> Self {
        Self::new()
    }
}

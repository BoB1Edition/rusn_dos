// Ver: 2 File: crate/x86/src/tracer.rs
use std::fs::File;
use std::io::{self, BufWriter, Write};

/// Трейт для трассировки выполнения инструкций.
pub trait Tracer {
    /// Логирует одну инструкцию (CS:IP и байты опкодов).
    fn log_instruction(&mut self, cs: u16, ip: u16, bytes: &[u8]);
    /// Принудительный сброс буфера на диск (нужно вызывать при HLT или краше).
    fn flush(&mut self) -> io::Result<()>;
}

/// Трейсер, который пишет лог опкодов в текстовый файл.
/// Использует BufWriter для минимизации системных вызовов (I/O bottleneck).
pub struct FileTracer {
    writer: BufWriter<File>,
}

impl FileTracer {
    /// Создает новый файловый трассировщик.
    /// `file_path` - путь к файлу лога (например, "logopcode.txt").
    pub fn new(file_path: &str) -> io::Result<Self> {
        let file = File::create(file_path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer })
    }
}

impl Tracer for FileTracer {
    #[inline]
    fn log_instruction(&mut self, cs: u16, ip: u16, bytes: &[u8]) {
        // Форматируем байты в hex-строку
        let hex_bytes: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
        
        // Записываем в буфер. Формат идентичен старому DosMachine:
        // CS:IP: BYTE1 BYTE2 BYTE3
        let _ = writeln!(
            self.writer,
            "{:#06x}:{:#06x}: {}",
            cs,
            ip,
            hex_bytes.join(" ")
        );
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

// Автоматический сброс буфера при уничтожении трассировщика
impl Drop for FileTracer {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// "Пустой" трассировщик. Используется, когда логирование отключено (no_log = true).
/// Компилятор оптимизирует вызовы его методов в пустоту (zero-cost abstraction).
pub struct NullTracer;

impl Tracer for NullTracer {
    #[inline(always)]
    fn log_instruction(&mut self, _cs: u16, _ip: u16, _bytes: &[u8]) {
        // Ничего не делаем
    }
    
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
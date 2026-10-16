// Ver: 1 File: crate/bus/src/display.rs

/// Абстракция окна вывода/ввода (минибуфер). Реализуется во внешнем crate.
/// Позволяет `bus`/`dos` не зависеть напрямую от конкретной GUI-библиотеки.
pub trait Display {
    fn is_open(&self) -> bool;
    fn size(&self) -> (usize, usize);
    /// Новые нажатия клавиш с прошлого вызова: (scancode, ascii).
    fn poll_keys(&mut self) -> Vec<(u8, u8)>;
    fn update(&mut self, pixels: &[u32], width: usize, height: usize);
}

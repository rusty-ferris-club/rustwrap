#![allow(clippy::module_name_repetitions)]

pub use console::style;
use console::Emoji;

pub static FINGER: Emoji<'_, '_> = Emoji("👉", "->");
pub static INFO: Emoji<'_, '_> = Emoji("🍭", "*");
pub static DOWNLOAD: Emoji<'_, '_> = Emoji("⚡️", "!");
pub static PKG: Emoji<'_, '_> = Emoji("📦", "*");
pub static COFFEE: Emoji<'_, '_> = Emoji("☕️", "*");
pub trait Console {
    fn say(&mut self, text: &str);
}

pub struct EnvConsole {}

impl Console for EnvConsole {
    fn say(&mut self, text: &str) {
        eprintln!("{text}");
    }
}

#[derive(Default)]
pub struct MemConsole {
    pub buffer: Vec<String>,
}
impl Console for MemConsole {
    fn say(&mut self, text: &str) {
        self.buffer.push(text.to_string());
    }
}

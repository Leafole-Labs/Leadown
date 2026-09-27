//! Markdown flag constants and list continuation helpers.
//!
//! The heavy tree-sitter parsing was removed with the GPUI migration. This
//! module now holds only the flag bitmask values used by the syntax highlighter
//! and the `list_prefix` helper for Enter-continuation in the editor.

pub const BOLD: u16 = 1;
pub const ITALIC: u16 = 2;
pub const UNDERLINE: u16 = 4;
pub const STRIKE: u16 = 8;
pub const CODE: u16 = 16;
pub const LINK: u16 = 32;
pub const MARK: u16 = 64;
pub const MUTED: u16 = 128;
pub const TASK: u16 = 256;
pub const DONE: u16 = 512;
pub const KEYWORD: u16 = 1024;
pub const STRING: u16 = 2048;
pub const COMMENT: u16 = 4096;
pub const NUMBER: u16 = 8192;

/// Continuation for Enter on a list/quote line: `(prefix byte len, next prefix)`.
pub fn list_prefix(line: &str) -> Option<(usize, String)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    let pad = &line[..indent];
    for task in ["- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] "] {
        if rest.starts_with(task) {
            return Some((indent + task.len(), format!("{pad}{}[ ] ", &task[..2])));
        }
    }
    for marker in ["- ", "* ", "+ ", "> "] {
        if rest.starts_with(marker) {
            return Some((indent + 2, format!("{pad}{marker}")));
        }
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && rest[digits..].starts_with(". ") {
        let n: u64 = rest[..digits].parse().ok()?;
        return Some((indent + digits + 2, format!("{pad}{}. ", n + 1)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_continuation() {
        assert_eq!(list_prefix("- item"), Some((2, "- ".into())));
        assert_eq!(list_prefix("  9. x"), Some((5, "  10. ".into())));
        assert_eq!(list_prefix("- [x] done"), Some((6, "- [ ] ".into())));
        assert_eq!(list_prefix("plain"), None);
    }
}

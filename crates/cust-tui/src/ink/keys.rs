//! Keyboard input decoding.
//!
//! Port of the parsing core of `pi-tui`'s `src/keys.ts`: raw stdin bytes in,
//! a structured [`Key`] out. Covers control characters, CSI/SS3 sequences with
//! xterm-style modifier parameters, and the Kitty keyboard protocol's
//! event-type field (press / repeat / release).

use std::fmt;

/// Modifier keys held during a key event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

/// What kind of key transition this event represents.
///
/// Only the Kitty protocol reports anything but [`KeyEventType::Press`];
/// without it every event decodes as a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyEventType {
    #[default]
    Press,
    Repeat,
    Release,
}

/// The key itself, independent of modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Escape,
    Delete,
    Insert,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
    /// A sequence recognized as input but not mapped to a named key.
    Unknown(String),
}

/// A decoded key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub code: KeyCode,
    pub modifiers: Modifiers,
    pub event: KeyEventType,
    /// The raw bytes this key was decoded from.
    pub raw: String,
}

impl Key {
    pub fn is_release(&self) -> bool {
        self.event == KeyEventType::Release
    }

    pub fn is_repeat(&self) -> bool {
        self.event == KeyEventType::Repeat
    }

    /// Match against a keybinding string like `"ctrl+c"`, `"shift+tab"`, `"a"`.
    ///
    /// Comparison is case-insensitive on the key name; unparseable specs never
    /// match rather than panicking, since bindings come from user config.
    pub fn matches(&self, spec: &str) -> bool {
        let mut wanted = Modifiers::default();
        let mut name = "";
        for part in spec.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => wanted.ctrl = true,
                "alt" | "meta" | "option" => wanted.alt = true,
                "shift" => wanted.shift = true,
                "super" | "cmd" => wanted.super_key = true,
                _ => name = part.trim(),
            }
        }
        if self.modifiers != wanted {
            return false;
        }
        match parse_key_name(name) {
            Some(code) => self.code == code,
            None => false,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.ctrl {
            write!(f, "ctrl+")?;
        }
        if self.modifiers.alt {
            write!(f, "alt+")?;
        }
        if self.modifiers.shift {
            write!(f, "shift+")?;
        }
        if self.modifiers.super_key {
            write!(f, "super+")?;
        }
        match &self.code {
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::F(n) => write!(f, "f{n}"),
            KeyCode::Unknown(_) => write!(f, "unknown"),
            other => write!(f, "{}", format!("{other:?}").to_ascii_lowercase()),
        }
    }
}

fn parse_key_name(name: &str) -> Option<KeyCode> {
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "escape" | "esc" => KeyCode::Escape,
        "delete" | "del" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        _ => {
            if let Some(n) = lower.strip_prefix('f') {
                if let Ok(n) = n.parse::<u8>() {
                    if (1..=20).contains(&n) {
                        return Some(KeyCode::F(n));
                    }
                }
            }
            let mut chars = name.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(c)
        }
    })
}

/// Decode every key event in a chunk of stdin bytes.
///
/// A single read can carry several keys (fast typing, paste, or a terminal
/// batching sequences), so this returns all of them in order.
pub fn parse_keys(data: &str) -> Vec<Key> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let (key, consumed) = parse_one(data, i);
        if consumed == 0 {
            break;
        }
        out.push(key);
        i += consumed;
    }
    out
}

/// Decode the first key event in `data`, if any.
pub fn parse_key(data: &str) -> Option<Key> {
    parse_keys(data).into_iter().next()
}

fn parse_one(data: &str, start: usize) -> (Key, usize) {
    let rest = &data[start..];
    let mut chars = rest.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => {
            return (
                Key {
                    code: KeyCode::Unknown(String::new()),
                    modifiers: Modifiers::default(),
                    event: KeyEventType::Press,
                    raw: String::new(),
                },
                0,
            )
        }
    };

    if first == '\u{1b}' {
        if let Some((key, len)) = parse_escape(rest) {
            return (key, len);
        }
        // Lone ESC, or ESC followed by a printable char = alt+char.
        if let Some(next) = chars.next() {
            if next != '[' && next != 'O' && !next.is_control() {
                return (
                    Key {
                        code: KeyCode::Char(next),
                        modifiers: Modifiers {
                            alt: true,
                            ..Default::default()
                        },
                        event: KeyEventType::Press,
                        raw: rest[..1 + next.len_utf8()].to_string(),
                    },
                    1 + next.len_utf8(),
                );
            }
        }
        return (simple(KeyCode::Escape, Modifiers::default(), "\u{1b}"), 1);
    }

    // C0 control characters.
    let key = match first {
        '\r' | '\n' => simple(KeyCode::Enter, Modifiers::default(), &first.to_string()),
        '\t' => simple(KeyCode::Tab, Modifiers::default(), "\t"),
        '\u{7f}' | '\u{8}' => simple(KeyCode::Backspace, Modifiers::default(), &first.to_string()),
        c if (c as u32) < 0x20 => {
            // Ctrl+letter maps to 0x01..0x1a; ctrl+space arrives as NUL.
            let ch = if c == '\0' {
                ' '
            } else {
                char::from(b'a' + (c as u8) - 1)
            };
            Key {
                code: KeyCode::Char(ch),
                modifiers: Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                event: KeyEventType::Press,
                raw: c.to_string(),
            }
        }
        c => simple(KeyCode::Char(c), Modifiers::default(), &c.to_string()),
    };
    (key, first.len_utf8())
}

fn simple(code: KeyCode, modifiers: Modifiers, raw: &str) -> Key {
    Key {
        code,
        modifiers,
        event: KeyEventType::Press,
        raw: raw.to_string(),
    }
}

/// Parse a CSI (`ESC [`) or SS3 (`ESC O`) sequence.
fn parse_escape(rest: &str) -> Option<(Key, usize)> {
    let bytes = rest.as_bytes();
    if bytes.len() < 3 {
        return None;
    }

    // SS3: ESC O <letter> — cursor keys in application mode, plus F1-F4.
    if bytes[1] == b'O' {
        let final_byte = bytes[2] as char;
        let code = match final_byte {
            'A' => KeyCode::Up,
            'B' => KeyCode::Down,
            'C' => KeyCode::Right,
            'D' => KeyCode::Left,
            'H' => KeyCode::Home,
            'F' => KeyCode::End,
            'P' => KeyCode::F(1),
            'Q' => KeyCode::F(2),
            'R' => KeyCode::F(3),
            'S' => KeyCode::F(4),
            _ => return None,
        };
        return Some((simple(code, Modifiers::default(), &rest[..3]), 3));
    }

    if bytes[1] != b'[' {
        return None;
    }

    // Find the final byte in @..~ that terminates the CSI sequence.
    let final_idx = bytes[2..].iter().position(|b| (0x40..=0x7e).contains(b))? + 2;
    let params = &rest[2..final_idx];
    let final_byte = bytes[final_idx] as char;
    let len = final_idx + 1;

    // Params are `a:b;c:d` — semicolons separate, colons sub-divide. The Kitty
    // protocol puts the event type in the modifier field's second sub-param.
    let groups: Vec<&str> = params.split(';').collect();
    let modifier_group = groups.get(1).copied().unwrap_or("");
    let mut sub = modifier_group.split(':');
    let modifier_value: u8 = sub.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let event = match sub.next().and_then(|s| s.parse::<u8>().ok()) {
        Some(2) => KeyEventType::Repeat,
        Some(3) => KeyEventType::Release,
        _ => KeyEventType::Press,
    };
    let modifiers = decode_modifier_param(modifier_value);

    let code = match final_byte {
        'A' => KeyCode::Up,
        'B' => KeyCode::Down,
        'C' => KeyCode::Right,
        'D' => KeyCode::Left,
        'H' => KeyCode::Home,
        'F' => KeyCode::End,
        'Z' => {
            // CSI Z is shift+tab regardless of the modifier field.
            return Some((
                Key {
                    code: KeyCode::Tab,
                    modifiers: Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                    event,
                    raw: rest[..len].to_string(),
                },
                len,
            ));
        }
        'u' => {
            // Kitty: CSI <codepoint> ; <mods> u
            let cp: u32 = groups.first()?.split(':').next()?.parse().ok()?;
            KeyCode::Char(char::from_u32(cp)?)
        }
        '~' => {
            let n: u8 = groups.first()?.split(':').next()?.parse().ok()?;
            match n {
                2 => KeyCode::Insert,
                3 => KeyCode::Delete,
                5 => KeyCode::PageUp,
                6 => KeyCode::PageDown,
                7 => KeyCode::Home,
                8 => KeyCode::End,
                11..=15 => KeyCode::F(n - 10),
                // 16 is unassigned in the xterm table; F6-F10 resume at 17.
                17..=21 => KeyCode::F(n - 11),
                23..=26 => KeyCode::F(n - 12),
                _ => KeyCode::Unknown(rest[..len].to_string()),
            }
        }
        _ => KeyCode::Unknown(rest[..len].to_string()),
    };

    Some((
        Key {
            code,
            modifiers,
            event,
            raw: rest[..len].to_string(),
        },
        len,
    ))
}

/// Decode an xterm modifier parameter: the value is a 1-based bitfield.
fn decode_modifier_param(value: u8) -> Modifiers {
    let bits = value.saturating_sub(1);
    Modifiers {
        shift: bits & 1 != 0,
        alt: bits & 2 != 0,
        ctrl: bits & 4 != 0,
        super_key: bits & 8 != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(s: &str) -> Key {
        parse_key(s).expect("expected a key")
    }

    #[test]
    fn decodes_plain_characters() {
        let k = one("a");
        assert_eq!(k.code, KeyCode::Char('a'));
        assert_eq!(k.modifiers, Modifiers::default());
    }

    #[test]
    fn decodes_control_characters() {
        assert_eq!(one("\u{3}").code, KeyCode::Char('c'));
        assert!(one("\u{3}").modifiers.ctrl);
        assert_eq!(one("\r").code, KeyCode::Enter);
        assert_eq!(one("\t").code, KeyCode::Tab);
        assert_eq!(one("\u{7f}").code, KeyCode::Backspace);
    }

    #[test]
    fn ctrl_space_arrives_as_nul() {
        let k = one("\0");
        assert_eq!(k.code, KeyCode::Char(' '));
        assert!(k.modifiers.ctrl);
    }

    #[test]
    fn decodes_arrow_keys_in_both_modes() {
        assert_eq!(one("\u{1b}[A").code, KeyCode::Up);
        assert_eq!(one("\u{1b}OA").code, KeyCode::Up);
    }

    #[test]
    fn decodes_xterm_modifier_parameters() {
        // CSI 1;5C = ctrl+right
        let k = one("\u{1b}[1;5C");
        assert_eq!(k.code, KeyCode::Right);
        assert!(k.modifiers.ctrl);
        assert!(!k.modifiers.shift);
    }

    #[test]
    fn decodes_shift_tab() {
        let k = one("\u{1b}[Z");
        assert_eq!(k.code, KeyCode::Tab);
        assert!(k.modifiers.shift);
    }

    #[test]
    fn decodes_function_and_navigation_keys() {
        assert_eq!(one("\u{1b}[3~").code, KeyCode::Delete);
        assert_eq!(one("\u{1b}[5~").code, KeyCode::PageUp);
        assert_eq!(one("\u{1b}[15~").code, KeyCode::F(5));
        assert_eq!(one("\u{1b}[17~").code, KeyCode::F(6));
        assert_eq!(one("\u{1b}[24~").code, KeyCode::F(12));
    }

    #[test]
    fn decodes_alt_prefixed_characters() {
        let k = one("\u{1b}b");
        assert_eq!(k.code, KeyCode::Char('b'));
        assert!(k.modifiers.alt);
    }

    #[test]
    fn lone_escape_is_escape() {
        assert_eq!(one("\u{1b}").code, KeyCode::Escape);
    }

    #[test]
    fn decodes_kitty_event_types() {
        // CSI 97 ; 1:3 u = release of 'a'
        let k = one("\u{1b}[97;1:3u");
        assert_eq!(k.code, KeyCode::Char('a'));
        assert!(k.is_release());
        let k = one("\u{1b}[97;1:2u");
        assert!(k.is_repeat());
        let k = one("\u{1b}[97;1:1u");
        assert_eq!(k.event, KeyEventType::Press);
    }

    #[test]
    fn splits_a_batched_read_into_several_keys() {
        let keys = parse_keys("ab\u{1b}[A");
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[2].code, KeyCode::Up);
    }

    #[test]
    fn matches_keybinding_specs() {
        assert!(one("\u{3}").matches("ctrl+c"));
        assert!(!one("\u{3}").matches("c"));
        assert!(one("\u{1b}[Z").matches("shift+tab"));
        assert!(one("\u{1b}[A").matches("up"));
        assert!(one("a").matches("a"));
    }

    #[test]
    fn unparseable_spec_never_matches() {
        assert!(!one("a").matches("not-a-key"));
        assert!(!one("a").matches(""));
    }

    #[test]
    fn multibyte_characters_survive_decoding() {
        let keys = parse_keys("日本");
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].code, KeyCode::Char('日'));
    }

    #[test]
    fn display_round_trips_common_bindings() {
        assert_eq!(one("\u{3}").to_string(), "ctrl+c");
        assert_eq!(one("\u{1b}[A").to_string(), "up");
    }
}

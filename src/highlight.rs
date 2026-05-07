use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "if", "else", "while", "for", "loop", "match", "return", "break",
    "continue", "struct", "enum", "impl", "trait", "pub", "mod", "use", "crate", "self", "super",
    "as", "in", "ref", "move", "async", "await", "dyn", "where", "type", "const", "static",
    "unsafe", "extern", "true", "false", "macro_rules",
];

const TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
    "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box", "Rc", "Arc",
    "HashMap", "HashSet", "BTreeMap", "BTreeSet", "Self",
];

/// Phosphor green (#00FF41) base
const GREEN: Color = Color::Rgb(0, 255, 65);
const DIM_GREEN: Color = Color::Rgb(0, 180, 45);
const KEYWORD_GREEN: Color = Color::Rgb(80, 255, 120);
const TYPE_CYAN: Color = Color::Rgb(0, 255, 220);
const STRING_YELLOW: Color = Color::Rgb(255, 220, 80);
const COMMENT_DIM: Color = Color::Rgb(100, 120, 100);
const NUMBER_ORANGE: Color = Color::Rgb(255, 180, 60);
const MACRO_MAGENTA: Color = Color::Rgb(220, 120, 255);

pub fn highlight_line(line: &str) -> Line<'static> {
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut i = 0;

    while i < len {
        // Line comment
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            let rest: String = chars[i..].iter().collect();
            spans.push(Span::styled(rest, Style::default().fg(COMMENT_DIM)));
            break;
        }

        // String literal
        if chars[i] == '"' {
            let mut j = i + 1;
            while j < len {
                if chars[j] == '\\' && j + 1 < len {
                    j += 2;
                    continue;
                }
                if chars[j] == '"' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            let s: String = chars[i..j].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(STRING_YELLOW)));
            i = j;
            continue;
        }

        // Char literal
        if chars[i] == '\'' && i + 2 < len {
            // Could be a char literal or a lifetime — check context
            let mut j = i + 1;
            if j < len && chars[j] == '\\' {
                j += 1;
            }
            if j < len {
                j += 1;
            }
            if j < len && chars[j] == '\'' {
                j += 1;
                let s: String = chars[i..j].iter().collect();
                spans.push(Span::styled(s, Style::default().fg(STRING_YELLOW)));
                i = j;
                continue;
            }
        }

        // Numbers
        if chars[i].is_ascii_digit()
            && (i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_')
        {
            let mut j = i;
            if j + 1 < len && chars[j] == '0' && (chars[j + 1] == 'x' || chars[j + 1] == 'b' || chars[j + 1] == 'o') {
                j += 2;
            }
            while j < len && (chars[j].is_ascii_alphanumeric() || chars[j] == '_' || chars[j] == '.') {
                j += 1;
            }
            let s: String = chars[i..j].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(NUMBER_ORANGE)));
            i = j;
            continue;
        }

        // Identifiers / keywords / types
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let mut j = i;
            while j < len && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let word: String = chars[i..j].iter().collect();

            // Check for macro invocation (word followed by !)
            if j < len && chars[j] == '!' {
                let mac: String = chars[i..=j].iter().collect();
                spans.push(Span::styled(mac, Style::default().fg(MACRO_MAGENTA)));
                i = j + 1;
                continue;
            }

            if KEYWORDS.contains(&word.as_str()) {
                spans.push(Span::styled(word, Style::default().fg(KEYWORD_GREEN)));
            } else if TYPES.contains(&word.as_str()) {
                spans.push(Span::styled(word, Style::default().fg(TYPE_CYAN)));
            } else {
                spans.push(Span::styled(word, Style::default().fg(GREEN)));
            }
            i = j;
            continue;
        }

        // Operators and punctuation
        let ch = chars[i];
        let s = ch.to_string();
        spans.push(Span::styled(s, Style::default().fg(DIM_GREEN)));
        i += 1;
    }

    if spans.is_empty() {
        spans.push(Span::styled(String::new(), Style::default().fg(GREEN)));
    }

    Line::from(spans)
}

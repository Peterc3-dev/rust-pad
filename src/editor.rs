use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone)]
pub struct Editor {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn set_content(&mut self, text: &str) {
        self.lines = text.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert_char(ch);
        }
    }

    fn current_line_len(&self) -> usize {
        self.lines[self.cursor_row].len()
    }

    fn clamp_col(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col > len {
            self.cursor_col = len;
        }
    }

    pub fn ensure_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        }
        if self.cursor_row >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor_row - visible_height + 1;
        }
    }

    fn insert_char(&mut self, ch: char) {
        let col = self.cursor_col;
        self.lines[self.cursor_row].insert(col, ch);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let col = self.cursor_col;
        let rest = self.lines[self.cursor_row][col..].to_string();
        self.lines[self.cursor_row].truncate(col);

        // Auto-indent: carry over leading whitespace from current line
        let indent: String = self.lines[self.cursor_row]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        let new_line = format!("{indent}{rest}");
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, new_line);
        self.cursor_col = indent.len();
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row > 0 {
            let removed = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&removed);
        }
    }

    fn delete(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col < len {
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Ctrl shortcuts handled elsewhere
                } else {
                    self.insert_char(c);
                }
            }
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.word_left();
                } else if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.current_line_len();
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.word_right();
                } else if self.cursor_col < self.current_line_len() {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
            KeyCode::Up if self.cursor_row > 0 => {
                self.cursor_row -= 1;
                self.clamp_col();
            }
            KeyCode::Down if self.cursor_row + 1 < self.lines.len() => {
                self.cursor_row += 1;
                self.clamp_col();
            }
            KeyCode::Home => {
                self.cursor_col = 0;
            }
            KeyCode::End => {
                self.cursor_col = self.current_line_len();
            }
            KeyCode::PageUp => {
                self.cursor_row = self.cursor_row.saturating_sub(20);
                self.clamp_col();
            }
            KeyCode::PageDown => {
                self.cursor_row = (self.cursor_row + 20).min(self.lines.len() - 1);
                self.clamp_col();
            }
            _ => {}
        }
    }

    fn word_left(&mut self) {
        if self.cursor_col == 0 {
            if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.current_line_len();
            }
            return;
        }
        let line = &self.lines[self.cursor_row];
        let chars: Vec<char> = line.chars().collect();
        let mut i = self.cursor_col;
        // Skip whitespace
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }
        // Skip word chars
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }
        self.cursor_col = i;
    }

    fn word_right(&mut self) {
        let len = self.current_line_len();
        if self.cursor_col >= len {
            if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
            return;
        }
        let line = &self.lines[self.cursor_row];
        let chars: Vec<char> = line.chars().collect();
        let mut i = self.cursor_col;
        // Skip word chars
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        self.cursor_col = i;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_editor_is_empty() {
        let ed = Editor::new();
        assert!(ed.is_empty());
        assert_eq!(ed.content(), "");
        assert_eq!(ed.cursor_row, 0);
        assert_eq!(ed.cursor_col, 0);
    }

    #[test]
    fn set_content_splits_into_lines() {
        let mut ed = Editor::new();
        ed.set_content("a\nb\nc");
        assert_eq!(ed.lines, vec!["a", "b", "c"]);
        assert_eq!(ed.content(), "a\nb\nc");
        assert!(!ed.is_empty());
        // Cursor resets to the top.
        assert_eq!((ed.cursor_row, ed.cursor_col), (0, 0));
    }

    #[test]
    fn set_content_empty_keeps_one_line() {
        let mut ed = Editor::new();
        ed.set_content("");
        assert_eq!(ed.lines, vec![String::new()]);
        assert!(ed.is_empty());
    }

    #[test]
    fn insert_str_advances_cursor() {
        let mut ed = Editor::new();
        ed.insert_str("abc");
        assert_eq!(ed.content(), "abc");
        assert_eq!(ed.cursor_col, 3);
    }

    #[test]
    fn newline_carries_leading_indent() {
        let mut ed = Editor::new();
        ed.insert_str("    foo");
        ed.insert_newline();
        assert_eq!(ed.lines, vec!["    foo", "    "]);
        // Cursor sits after the carried-over indentation.
        assert_eq!(ed.cursor_row, 1);
        assert_eq!(ed.cursor_col, 4);
    }

    #[test]
    fn newline_splits_line_at_cursor() {
        let mut ed = Editor::new();
        ed.set_content("abcd");
        ed.cursor_col = 2;
        ed.insert_newline();
        assert_eq!(ed.lines, vec!["ab", "cd"]);
    }

    #[test]
    fn backspace_joins_lines_at_start_of_line() {
        let mut ed = Editor::new();
        ed.set_content("ab\ncd");
        ed.cursor_row = 1;
        ed.cursor_col = 0;
        ed.backspace();
        assert_eq!(ed.lines, vec!["abcd"]);
        assert_eq!(ed.cursor_row, 0);
        assert_eq!(ed.cursor_col, 2);
    }

    #[test]
    fn delete_merges_next_line_at_end_of_line() {
        let mut ed = Editor::new();
        ed.set_content("ab\ncd");
        ed.cursor_row = 0;
        ed.cursor_col = 2;
        ed.delete();
        assert_eq!(ed.lines, vec!["abcd"]);
    }
}

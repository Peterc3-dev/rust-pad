use std::path::PathBuf;

use crate::editor::Editor;
use crate::runner::{self, RunResult};
use crate::snippets;
use crate::templates;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    Editing,
    SavePrompt,
    LoadBrowser,
    HistoryBrowser,
}

pub struct App {
    pub editor: Editor,
    pub mode: Mode,
    pub output_lines: Vec<OutputLine>,
    pub status_msg: String,
    pub save_input: String,
    pub browser_items: Vec<(String, PathBuf)>,
    pub browser_index: usize,
    pub template_index: usize,
}

#[derive(Clone)]
pub struct OutputLine {
    pub text: String,
    pub kind: OutputKind,
}

#[derive(Clone, Copy, PartialEq)]
pub enum OutputKind {
    Normal,
    Error,
    Warning,
    Success,
    Info,
}

impl App {
    pub fn new() -> Self {
        Self {
            editor: Editor::new(),
            mode: Mode::Editing,
            output_lines: vec![OutputLine {
                text: "rust-pad ready. F5=Run  F6=Check  F2=Save  F3=Load  F4=History  Ctrl+Q=Quit"
                    .into(),
                kind: OutputKind::Info,
            }],
            status_msg: String::new(),
            save_input: String::new(),
            browser_items: Vec::new(),
            browser_index: 0,
            template_index: 0,
        }
    }

    pub fn compile_and_run(&mut self) {
        let code = self.editor.content();
        if code.trim().is_empty() {
            self.set_output_info("Nothing to run.");
            return;
        }

        self.set_output_info("Compiling and running...");
        snippets::save_history(&code).ok();

        let result = runner::compile_and_run(&code);
        self.display_result(&result);
    }

    pub fn compile_only(&mut self) {
        let code = self.editor.content();
        if code.trim().is_empty() {
            self.set_output_info("Nothing to compile.");
            return;
        }

        self.set_output_info("Compiling...");

        let result = runner::compile(&code);
        self.display_result(&result);
    }

    fn display_result(&mut self, result: &RunResult) {
        self.output_lines.clear();

        // Timing
        self.output_lines.push(OutputLine {
            text: format!("--- Elapsed: {}ms ---", result.elapsed_ms),
            kind: OutputKind::Info,
        });

        // Compiler output
        if !result.compiler_output.is_empty() {
            for line in result.compiler_output.lines() {
                let kind = if line.contains("error") {
                    OutputKind::Error
                } else if line.contains("warning") {
                    OutputKind::Warning
                } else if line.contains("note:") || line.starts_with('[') {
                    OutputKind::Info
                } else {
                    OutputKind::Normal
                };
                self.output_lines.push(OutputLine {
                    text: line.to_string(),
                    kind,
                });
            }
        }

        if result.compile_success {
            if !result.compiler_output.trim().is_empty() {
                self.output_lines.push(OutputLine {
                    text: String::new(),
                    kind: OutputKind::Normal,
                });
            }
            self.output_lines.push(OutputLine {
                text: "--- Compilation succeeded ---".into(),
                kind: OutputKind::Success,
            });
        } else {
            self.output_lines.push(OutputLine {
                text: "--- Compilation FAILED ---".into(),
                kind: OutputKind::Error,
            });
            return;
        }

        // Program output
        if !result.program_output.is_empty() {
            self.output_lines.push(OutputLine {
                text: String::new(),
                kind: OutputKind::Normal,
            });
            for line in result.program_output.lines() {
                let kind = if line.starts_with("[stderr]")
                    || line.starts_with("[exit code:")
                    || line.starts_with("[killed")
                {
                    OutputKind::Error
                } else {
                    OutputKind::Success
                };
                self.output_lines.push(OutputLine {
                    text: line.to_string(),
                    kind,
                });
            }
        }

        if result.success {
            self.status_msg = "Run completed successfully".into();
        } else if result.compile_success {
            self.status_msg = "Program exited with error".into();
        } else {
            self.status_msg = "Compilation failed".into();
        }
    }

    fn set_output_info(&mut self, msg: &str) {
        self.output_lines = vec![OutputLine {
            text: msg.to_string(),
            kind: OutputKind::Info,
        }];
        self.status_msg = msg.to_string();
    }

    pub fn start_save(&mut self) {
        self.mode = Mode::SavePrompt;
        self.save_input.clear();
        self.status_msg = "Enter snippet name (Enter to confirm, Esc to cancel):".into();
    }

    pub fn confirm_save(&mut self) {
        let name = self.save_input.clone();
        if name.trim().is_empty() {
            self.status_msg = "Save cancelled (empty name).".into();
            self.mode = Mode::Editing;
            return;
        }

        let content = self.editor.content();
        match snippets::save_snippet(&name, &content) {
            Ok(path) => {
                self.status_msg = format!("Saved to {}", path.display());
            }
            Err(e) => {
                self.status_msg = format!("Save error: {e}");
            }
        }
        self.save_input.clear();
        self.mode = Mode::Editing;
    }

    pub fn open_load_browser(&mut self) {
        self.browser_items = snippets::list_snippets();
        self.browser_index = 0;
        if self.browser_items.is_empty() {
            self.status_msg = "No saved snippets found.".into();
        } else {
            self.mode = Mode::LoadBrowser;
            self.status_msg = "Select snippet (Enter=Load, Ctrl+D=Delete, Esc=Cancel)".into();
        }
    }

    pub fn open_history_browser(&mut self) {
        self.browser_items = snippets::list_history();
        self.browser_index = 0;
        if self.browser_items.is_empty() {
            self.status_msg = "No history entries found.".into();
        } else {
            self.mode = Mode::HistoryBrowser;
            self.status_msg = "Select history entry (Enter=Load, Ctrl+D=Delete, Esc=Cancel)".into();
        }
    }

    pub fn load_selected_browser_item(&mut self) {
        if let Some((_name, path)) = self.browser_items.get(self.browser_index) {
            match snippets::load_file(path) {
                Ok(content) => {
                    self.editor.set_content(&content);
                    self.status_msg = format!("Loaded: {}", path.display());
                }
                Err(e) => {
                    self.status_msg = format!("Load error: {e}");
                }
            }
        }
        self.mode = Mode::Editing;
    }

    pub fn delete_selected_browser_item(&mut self) {
        if let Some((_name, path)) = self.browser_items.get(self.browser_index).cloned() {
            match snippets::delete_file(&path) {
                Ok(()) => {
                    self.browser_items.remove(self.browser_index);
                    if self.browser_index >= self.browser_items.len() && self.browser_index > 0 {
                        self.browser_index -= 1;
                    }
                    self.status_msg = "Deleted.".into();
                    if self.browser_items.is_empty() {
                        self.mode = Mode::Editing;
                    }
                }
                Err(e) => {
                    self.status_msg = format!("Delete error: {e}");
                }
            }
        }
    }

    pub fn cycle_template(&mut self) {
        let templates = templates::TEMPLATES;
        if templates.is_empty() {
            return;
        }
        let (name, code) = templates[self.template_index % templates.len()];
        self.editor.set_content(code);
        self.status_msg = format!("Template: {name} (Tab to cycle on empty editor)");
        self.template_index += 1;
    }
}

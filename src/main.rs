mod app;
mod editor;
mod highlight;
mod runner;
mod snippets;
mod templates;
mod ui;

use std::io;

use app::App;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

#[derive(Parser)]
#[command(name = "rust-pad", about = "Terminal Rust scratchpad/REPL")]
struct Cli {
    /// Load a file into the editor on startup
    #[arg(short, long)]
    file: Option<String>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    if let Some(path) = cli.file {
        if let Ok(content) = std::fs::read_to_string(&path) {
            app.editor.set_content(&content);
        }
    }

    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if app.mode == app::Mode::SavePrompt {
                match key.code {
                    KeyCode::Enter => {
                        app.confirm_save();
                    }
                    KeyCode::Esc => {
                        app.mode = app::Mode::Editing;
                        app.save_input.clear();
                    }
                    KeyCode::Backspace => {
                        app.save_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.save_input.push(c);
                    }
                    _ => {}
                }
                continue;
            }

            if app.mode == app::Mode::LoadBrowser || app.mode == app::Mode::HistoryBrowser {
                match key.code {
                    KeyCode::Esc => {
                        app.mode = app::Mode::Editing;
                    }
                    KeyCode::Up => {
                        if app.browser_index > 0 {
                            app.browser_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if app.browser_index + 1 < app.browser_items.len() {
                            app.browser_index += 1;
                        }
                    }
                    KeyCode::Enter => {
                        app.load_selected_browser_item();
                    }
                    KeyCode::Char('d')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.delete_selected_browser_item();
                    }
                    _ => {}
                }
                continue;
            }

            // Normal editing mode
            match key.code {
                KeyCode::F(5) => app.compile_and_run(),
                KeyCode::F(6) => app.compile_only(),
                KeyCode::F(2) => app.start_save(),
                KeyCode::F(3) => app.open_load_browser(),
                KeyCode::F(4) => app.open_history_browser(),
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Tab => {
                    if app.editor.is_empty() {
                        app.cycle_template();
                    } else {
                        app.editor.insert_str("    ");
                    }
                }
                _ => app.editor.handle_key(key),
            }
        }
    }
}

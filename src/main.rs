mod game;
mod ui;
mod wiki;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use game::{Game, Screen};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut game = Game::new();

    terminal.draw(|f| ui::draw(f, &game))?;
    game.init().await?;

    loop {
        game.tick();
        terminal.draw(|f| ui::draw(f, &game))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                // Ctrl+C always quits
                if ctrl && key.code == KeyCode::Char('c') {
                    break;
                }

                match &game.screen {
                    Screen::Loading(_) => {}
                    Screen::Playing => match key.code {
                        KeyCode::Esc => {
                            // Clear filter, or do nothing if no filter
                            if !game.filter.is_empty() {
                                game.filter.clear();
                                game.selected = 0;
                            }
                        }
                        KeyCode::Up => game.move_selection(-1),
                        KeyCode::Down => game.move_selection(1),
                        KeyCode::Tab => {
                            game.show_hints = !game.show_hints;
                            if game.show_hints && !game.hints_loaded {
                                terminal.draw(|f| ui::draw(f, &game))?;
                                game.load_hints().await;
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(title) = game.selected_link() {
                                game.navigate(&title).await?;
                            }
                        }
                        KeyCode::Backspace => {
                            if !game.filter.is_empty() {
                                game.filter.pop();
                                game.selected = 0;
                            }
                        }
                        KeyCode::Char('g') if ctrl => {
                            // Give up
                            game.give_up();
                            terminal.draw(|f| ui::draw(f, &game))?;
                            game.find_optimal().await;
                        }
                        KeyCode::Char(c) => {
                            game.filter.push(c);
                            game.selected = 0;
                        }
                        _ => {}
                    },
                    Screen::Won | Screen::GaveUp => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('r') => {
                            game = Game::new();
                            terminal.draw(|f| ui::draw(f, &game))?;
                            game.init().await?;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}

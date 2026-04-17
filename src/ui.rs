use crate::game::{Game, Screen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, game: &Game) {
    match &game.screen {
        Screen::Loading(msg) => draw_loading(f, msg),
        Screen::Playing => draw_playing(f, game),
        Screen::Won => draw_end(f, game, true),
        Screen::GaveUp => draw_end(f, game, false),
    }
}

fn draw_loading(f: &mut Frame, msg: &str) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" 🌐 wikirace ")
        .title_style(Style::default().fg(Color::Cyan).bold());

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(msg, Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(Span::styled(
            "⏳ Please wait...",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .block(block);

    f.render_widget(text, area);
}

fn draw_playing(f: &mut Frame, game: &Game) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(4), // target info
            Constraint::Length(5), // article info
            Constraint::Min(10),  // links
            Constraint::Length(3), // filter bar
            Constraint::Length(1), // help
        ])
        .split(f.area());

    draw_header(f, game, chunks[0]);
    draw_target(f, game, chunks[1]);
    draw_article(f, game, chunks[2]);
    draw_links(f, game, chunks[3]);
    draw_filter(f, game, chunks[4]);
    draw_help(f, game, chunks[5]);

    if let Some(err) = &game.error {
        draw_error_popup(f, err);
    }
}

fn draw_header(f: &mut Frame, game: &Game, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Min(20),
            Constraint::Length(14),
        ])
        .split(area);

    let steps = Paragraph::new(Line::from(vec![
        Span::styled(" Steps: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            game.steps.to_string(),
            Style::default().fg(Color::Yellow).bold(),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let target = Paragraph::new(Line::from(vec![
        Span::styled("🎯 ", Style::default()),
        Span::styled(
            &game.target_title,
            Style::default().fg(Color::Red).bold(),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Target ")
            .title_style(Style::default().fg(Color::Red)),
    );

    let time = Paragraph::new(Line::from(vec![
        Span::styled("⏱  ", Style::default()),
        Span::styled(
            game.format_time(),
            Style::default().fg(Color::Cyan).bold(),
        ),
    ]))
    .alignment(Alignment::Right)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(steps, cols[0]);
    f.render_widget(target, cols[1]);
    f.render_widget(time, cols[2]);
}

fn draw_target(f: &mut Frame, game: &Game, area: Rect) {
    let extract = if game.target_extract.len() > 150 {
        format!("{}...", &game.target_extract[..150])
    } else {
        game.target_extract.clone()
    };

    let text = Paragraph::new(extract)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(format!(" 🎯 {} ", game.target_title))
                .title_style(Style::default().fg(Color::Red).bold()),
        );

    f.render_widget(text, area);
}

fn draw_article(f: &mut Frame, game: &Game, area: Rect) {
    let Some(article) = &game.current else {
        return;
    };

    let extract = if article.extract.len() > 200 {
        format!("{}...", &article.extract[..200])
    } else {
        article.extract.clone()
    };

    let text = Paragraph::new(extract)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .title(format!(" 📖 {} ", article.title))
                .title_style(Style::default().fg(Color::Green).bold()),
        );

    f.render_widget(text, area);
}

fn draw_links(f: &mut Frame, game: &Game, area: Rect) {
    let links = game.filtered_links();
    let total = links.len();

    let visible_height = area.height.saturating_sub(2) as usize;
    let offset = if game.selected >= game.link_offset + visible_height {
        game.selected.saturating_sub(visible_height - 1)
    } else if game.selected < game.link_offset {
        game.selected
    } else {
        game.link_offset
    };

    let items: Vec<ListItem> = links
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .map(|(i, link)| {
            let is_target = link.to_lowercase() == game.target_title.to_lowercase();
            let is_hint = game.is_hint_link(link);
            let is_selected = i == game.selected;

            let style = if is_selected && is_target {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_target {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if is_hint {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if is_target {
                "🎯 "
            } else if is_hint {
                "🔥 "
            } else if is_selected {
                "▸  "
            } else {
                "   "
            };

            ListItem::new(Line::from(Span::styled(
                format!("{prefix}{link}"),
                style,
            )))
        })
        .collect();

    let hint_status = if game.show_hints { " 💡 ON" } else { "" };
    let title = format!(" Links ({total}){hint_status} ");
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .title(title)
            .title_style(Style::default().fg(Color::Magenta)),
    );

    f.render_widget(list, area);
}

fn draw_filter(f: &mut Frame, game: &Game, area: Rect) {
    let display = if game.filter.is_empty() {
        Span::styled(
            "Type to filter links...",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::styled(&game.filter, Style::default().fg(Color::Yellow))
    };

    let filter = Paragraph::new(Line::from(vec![
        Span::styled(" 🔍 ", Style::default()),
        display,
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Filter ")
            .title_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(filter, area);
}

fn draw_help(f: &mut Frame, game: &Game, area: Rect) {
    let hint_label = if game.show_hints {
        "hide hints"
    } else {
        "hints"
    };
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓", Style::default().fg(Color::Cyan)),
        Span::styled(" navigate  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::styled(" follow  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::styled(" clear filter  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {hint_label}  "), Style::default().fg(Color::DarkGray)),
        Span::styled("Ctrl+G", Style::default().fg(Color::Red)),
        Span::styled(" give up  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Ctrl+C", Style::default().fg(Color::Red)),
        Span::styled(" quit", Style::default().fg(Color::DarkGray)),
    ]));

    f.render_widget(help, area);
}

fn draw_end(f: &mut Frame, game: &Game, won: bool) {
    let area = f.area();
    f.render_widget(Clear, area);

    let (title_text, title_color, banner) = if won {
        ("Victory!", Color::Green, "🎉  YOU WON!  🎉")
    } else {
        ("Game Over", Color::Red, "😔  GAVE UP  😔")
    };

    let path_display: Vec<Line> = game
        .path
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let arrow = if i < game.path.len() - 1 {
                " → "
            } else if won {
                " 🏁"
            } else {
                " ✗"
            };
            Line::from(vec![
                Span::styled(
                    format!("  {}: ", i),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(title, Style::default().fg(Color::Cyan).bold()),
                Span::styled(arrow, Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            banner,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Steps: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                game.steps.to_string(),
                Style::default().fg(Color::Yellow).bold(),
            ),
            Span::styled("    Time: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                game.format_time(),
                Style::default().fg(Color::Cyan).bold(),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Your path:",
            Style::default().fg(Color::White).bold(),
        )),
    ];
    lines.extend(path_display);

    // Optimal path section
    lines.push(Line::from(""));
    if game.searching_optimal {
        lines.push(Line::from(Span::styled(
            "  ⏳ Searching for optimal path (BFS, max depth 4)...",
            Style::default().fg(Color::Yellow),
        )));
    } else if let Some(optimal) = &game.optimal_path {
        lines.push(Line::from(Span::styled(
            format!("  Optimal path ({} steps):", optimal.len() - 1),
            Style::default().fg(Color::Green).bold(),
        )));
        for (i, title) in optimal.iter().enumerate() {
            let arrow = if i < optimal.len() - 1 {
                " → "
            } else {
                " 🏁"
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", i),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(title, Style::default().fg(Color::Green)),
                Span::styled(arrow, Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  No optimal path found within 4 steps",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press 'r' to play again or 'q' to quit",
        Style::default().fg(Color::DarkGray),
    )));

    let text = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(title_color))
                .title(format!(" 🌐 wikirace — {title_text} "))
                .title_style(Style::default().fg(title_color).bold()),
        );

    f.render_widget(text, area);
}

fn draw_error_popup(f: &mut Frame, msg: &str) {
    let area = f.area();
    let popup = Rect {
        x: area.width / 4,
        y: area.height / 2 - 2,
        width: area.width / 2,
        height: 5,
    };

    f.render_widget(Clear, popup);
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(msg, Style::default().fg(Color::Red))),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .title(" Error ")
            .title_style(Style::default().fg(Color::Red).bold()),
    );

    f.render_widget(text, popup);
}

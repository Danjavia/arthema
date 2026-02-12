use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem, Tabs},
    Frame,
};

use crate::app::{ActivePanel, App, EditorFocus, LeftPanelTab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header
            Constraint::Min(0),    // Main
            Constraint::Length(3), // Footer
        ])
        .split(f.size());

    // Header
    let header_art = " ▟████▙ ██████  ████████ ██      ██ ████████ ▟████▙ ▟████▙ 
▟█▘  █▙ ██   ██    ██    ██      ██ ██       ██    ██ ██    ██ 
████████ ██████     ██    ████████ ██████   ████████ ████████ 
██    ██ ██   ██    ██    ██      ██ ██       ██    ██ ██    ██ 
██    ██ ██   ██    ██    ██      ██ ████████ ██    ██ ██    ██ ";
    f.render_widget(Paragraph::new(header_art).style(Style::default().fg(Color::Cyan)), chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), 
            Constraint::Percentage(45), 
            Constraint::Percentage(30),
        ])
        .split(chunks[1]);

    // 1. Panel Izquierdo (Tabs: Collections / History)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(main_chunks[0]);

    let titles = vec![" COLLECTIONS ", " HISTORY "];
    let sel_idx = match app.left_panel_tab {
        LeftPanelTab::Collections => 0,
        LeftPanelTab::History => 1,
    };
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).border_style(get_border_style(&app.active_panel, ActivePanel::Collections)))
        .select(sel_idx)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Magenta));
    f.render_widget(tabs, left_chunks[0]);

    let items: Vec<ListItem> = match app.left_panel_tab {
        LeftPanelTab::Collections => app.collections.requests.iter().enumerate()
            .map(|(i, r)| {
                let style = if i == app.selected_idx && matches!(app.active_panel, ActivePanel::Collections) {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else { Style::default().fg(Color::Green) };
                ListItem::new(format!(" > {}", r.name)).style(style)
            }).collect(),
        LeftPanelTab::History => app.collections.history.iter().enumerate()
            .map(|(i, r)| {
                let style = if i == app.selected_idx && matches!(app.active_panel, ActivePanel::Collections) {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else { Style::default().fg(Color::DarkGray) };
                ListItem::new(format!(" [{}] {}", r.method, r.url)).style(style)
            }).collect(),
    };
    f.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).border_style(get_border_style(&app.active_panel, ActivePanel::Collections))), left_chunks[1]);

    // 2. Editor Panel
    let editor_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), 
            Constraint::Percentage(30), 
            Constraint::Min(0),
        ])
        .split(main_chunks[1]);

    app.url_rect = editor_area[0];
    app.headers_rect = editor_area[1];
    app.body_rect = editor_area[2];

    app.url_area.set_block(Block::default().title(format!(" ⚡ {} URL ", app.method)).borders(Borders::ALL).border_style(get_editor_border(app, EditorFocus::Url)));
    configure_cursor(app, EditorFocus::Url);
    f.render_widget(app.url_area.widget(), editor_area[0]);

    app.headers_area.set_block(Block::default().title(" 📋 HEADERS ").borders(Borders::ALL).border_style(get_editor_border(app, EditorFocus::Headers)));
    configure_cursor(app, EditorFocus::Headers);
    f.render_widget(app.headers_area.widget(), editor_area[1]);

    app.body_area.set_block(Block::default().title(" 📦 BODY ").borders(Borders::ALL).border_style(get_editor_border(app, EditorFocus::Body)));
    configure_cursor(app, EditorFocus::Body);
    f.render_widget(app.body_area.widget(), editor_area[2]);

    // 3. Response & AI
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[2]);

    f.render_widget(Paragraph::new(app.response.as_str()).block(Block::default().title(" 📡 RESPONSE ").borders(Borders::ALL).border_style(get_border_style(&app.active_panel, ActivePanel::Response))).scroll((app.response_scroll, 0)).wrap(Wrap { trim: true }), right_chunks[0]);
    f.render_widget(Paragraph::new(app.ai_response.as_str()).style(Style::default().fg(Color::Magenta)).block(Block::default().title(" 🧠 AI AGENT ").borders(Borders::ALL).border_style(get_border_style(&app.active_panel, ActivePanel::AI))).wrap(Wrap { trim: true }), right_chunks[1]);

    // Footer
    let footer_text = " [H] History/Coll | [F] Focus | [I] Insert | [C] Copy | [S] Save | [Enter] Run ";
    f.render_widget(Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray)).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::Magenta))), chunks[2]);
}

fn configure_cursor(app: &mut App, focus: EditorFocus) {
    let area = match focus {
        EditorFocus::Url => &mut app.url_area,
        EditorFocus::Headers => &mut app.headers_area,
        EditorFocus::Body => &mut app.body_area,
    };
    if app.input_mode && app.editor_focus == focus {
        area.set_cursor_style(Style::default().bg(Color::Yellow).fg(Color::Black));
    } else { area.set_cursor_style(Style::default()); }
}

fn get_editor_border(app: &App, focus: EditorFocus) -> Style {
    if matches!(app.active_panel, ActivePanel::Editor) && app.editor_focus == focus {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else { Style::default().fg(Color::Cyan) }
}

fn get_border_style(active: &ActivePanel, current: ActivePanel) -> Style {
    let is_active = match (active, current) {
        (ActivePanel::Collections, ActivePanel::Collections) => true,
        (ActivePanel::Editor, ActivePanel::Editor) => true,
        (ActivePanel::Response, ActivePanel::Response) => true,
        (ActivePanel::AI, ActivePanel::AI) => true,
        _ => false,
    };
    if is_active { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
    else { Style::default().fg(Color::Rgb(60, 60, 60)) }
}

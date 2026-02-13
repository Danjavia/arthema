use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
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

    // 1. Panel Izquierdo
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
        .block(Block::default().borders(Borders::ALL).border_style(get_border_style(app.active_panel, ActivePanel::Collections)))
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
    f.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).border_style(get_border_style(app.active_panel, ActivePanel::Collections))), left_chunks[1]);

    // 2. Editor Panel con Pestañas
    let editor_root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Barra de pestañas
            Constraint::Min(0),    // Contenido
        ])
        .split(main_chunks[1]);

    let tab_titles: Vec<Line> = app.tabs.iter().enumerate()
        .map(|(i, t)| {
            if i == app.active_tab {
                Line::from(vec![Span::styled(format!(" {} ", t.name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))])
            } else {
                Line::from(vec![Span::styled(format!(" {} ", t.name), Style::default().fg(Color::DarkGray))])
            }
        }).collect();
    
    let request_tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" 📂 OPEN REQUESTS ").border_style(get_border_style(app.active_panel, ActivePanel::Editor)))
        .select(app.active_tab)
        .highlight_style(Style::default().fg(Color::Yellow));
    f.render_widget(request_tabs, editor_root[0]);

    let editor_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), 
            Constraint::Percentage(30), 
            Constraint::Min(0),
        ])
        .split(editor_root[1]);

    app.url_rect = editor_area[0];
    app.headers_rect = editor_area[1];
    app.body_rect = editor_area[2];

    let input_mode = app.input_mode;
    let active_panel = app.active_panel;
    let active_tab_idx = app.active_tab;
    let tab = &mut app.tabs[active_tab_idx];
    
    tab.url_area.set_block(Block::default().title(format!(" ⚡ {} URL ", tab.method)).borders(Borders::ALL).border_style(get_editor_border(active_panel, tab.editor_focus, EditorFocus::Url)));
    if input_mode && tab.editor_focus == EditorFocus::Url { tab.url_area.set_cursor_style(Style::default().bg(Color::Yellow).fg(Color::Black)); }
    else { tab.url_area.set_cursor_style(Style::default()); }
    f.render_widget(tab.url_area.widget(), editor_area[0]);

    tab.headers_area.set_block(Block::default().title(" 📋 HEADERS ").borders(Borders::ALL).border_style(get_editor_border(active_panel, tab.editor_focus, EditorFocus::Headers)));
    if input_mode && tab.editor_focus == EditorFocus::Headers { tab.headers_area.set_cursor_style(Style::default().bg(Color::Yellow).fg(Color::Black)); }
    else { tab.headers_area.set_cursor_style(Style::default()); }
    f.render_widget(tab.headers_area.widget(), editor_area[1]);

    tab.body_area.set_block(Block::default().title(" 📦 BODY ").borders(Borders::ALL).border_style(get_editor_border(active_panel, tab.editor_focus, EditorFocus::Body)));
    if input_mode && tab.editor_focus == EditorFocus::Body { tab.body_area.set_cursor_style(Style::default().bg(Color::Yellow).fg(Color::Black)); }
    else { tab.body_area.set_cursor_style(Style::default()); }
    f.render_widget(tab.body_area.widget(), editor_area[2]);

    // 3. Response & AI
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[2]);

    f.render_widget(Paragraph::new(tab.response.as_str()).block(Block::default().title(" 📡 RESPONSE ").borders(Borders::ALL).border_style(get_border_style(active_panel, ActivePanel::Response))).scroll((tab.response_scroll, 0)).wrap(Wrap { trim: true }), right_chunks[0]);
    f.render_widget(Paragraph::new(app.ai_response.as_str()).style(Style::default().fg(Color::Magenta)).block(Block::default().title(" 🧠 AI AGENT ").borders(Borders::ALL).border_style(get_border_style(active_panel, ActivePanel::AI))).wrap(Wrap { trim: true }), right_chunks[1]);

    // Footer
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(70)])
        .split(chunks[2]);

    let footer_text = " [Ctrl+T] New | [Ctrl+W] Close | [N] Next | [M/Shift+M] Method ";
    f.render_widget(Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray)).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::Magenta))), footer_chunks[0]);

    let sys_metrics = Line::from(vec![
        Span::styled(format!(" ⚡ BAT: {} ", app.battery_level), Style::default().fg(Color::Cyan)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" CPU: {:.1}% ", app.cpu_usage), Style::default().fg(Color::Cyan)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" MEM: {}MB ", app.mem_used), Style::default().fg(Color::Cyan)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" APP: {:.1}% {}MB ", app.proc_cpu, app.proc_mem), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(sys_metrics).alignment(Alignment::Right).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::Magenta))), footer_chunks[1]);
}

fn get_editor_border(active_panel: ActivePanel, current_focus: EditorFocus, target_focus: EditorFocus) -> Style {
    if matches!(active_panel, ActivePanel::Editor) && current_focus == target_focus {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else { Style::default().fg(Color::Cyan) }
}

fn get_border_style(active: ActivePanel, current: ActivePanel) -> Style {
    if active == current { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
    else { Style::default().fg(Color::Rgb(60, 60, 60)) }
}

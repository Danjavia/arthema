use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap, List, ListItem, Tabs, Clear},
    Frame,
};

use crate::app::{ActivePanel, App, EditorFocus, BodyType};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Main area (reclamamos espacio del header)
            Constraint::Length(3), // Footer
        ])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), 
            Constraint::Percentage(45), 
            Constraint::Percentage(30),
        ])
        .split(chunks[0]);

    // 1. Panel Izquierdo
    let left_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(0)]).split(main_chunks[0]);
    let titles = vec![" COLLECTIONS ", " HISTORY "];
    let sel_idx = if matches!(app.left_panel_tab, crate::app::LeftPanelTab::Collections) { 0 } else { 1 };
    f.render_widget(Tabs::new(titles).block(Block::default().borders(Borders::ALL).border_style(get_border_style(app.active_panel, ActivePanel::Collections))).select(sel_idx).style(Style::default().fg(Color::Cyan)).highlight_style(Style::default().fg(Color::Black).bg(Color::Magenta)), left_chunks[0]);

    let items: Vec<ListItem> = match app.left_panel_tab {
        crate::app::LeftPanelTab::Collections => app.collections.requests.iter().enumerate().map(|(i, r)| {
            let style = if i == app.selected_idx && matches!(app.active_panel, ActivePanel::Collections) { Style::default().fg(Color::Black).bg(Color::Cyan) } else { Style::default().fg(Color::Green) };
            let prefix = r.group.as_ref().map(|g| format!("[{}] ", g)).unwrap_or_default();
            ListItem::new(format!(" > {}{}", prefix, r.name)).style(style)
        }).collect(),
        crate::app::LeftPanelTab::History => app.collections.history.iter().enumerate().map(|(i, r)| {
            let style = if i == app.selected_idx && matches!(app.active_panel, ActivePanel::Collections) { Style::default().fg(Color::Black).bg(Color::Cyan) } else { Style::default().fg(Color::DarkGray) };
            ListItem::new(format!(" [{}] {}", r.method, r.url)).style(style)
        }).collect(),
    };
    f.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).border_style(get_border_style(app.active_panel, ActivePanel::Collections))), left_chunks[1]);

    // 2. Editor Panel con Pestañas
    let editor_root = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(0)]).split(main_chunks[1]);
    let tab_titles: Vec<Line> = app.tabs.iter().enumerate().map(|(i, t)| {
        if i == app.active_tab { Line::from(vec![Span::styled(format!(" {} ", t.name), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]) }
        else { Line::from(vec![Span::styled(format!(" {} ", t.name), Style::default().fg(Color::DarkGray))]) }
    }).collect();
    f.render_widget(Tabs::new(tab_titles).block(Block::default().borders(Borders::ALL).title(" 📂 OPEN REQUESTS ").border_style(get_border_style(app.active_panel, ActivePanel::Editor))).select(app.active_tab).highlight_style(Style::default().fg(Color::Yellow)), editor_root[0]);

    let editor_area = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(3), // URL
        Constraint::Length(3), // Body Type Toggle
        Constraint::Percentage(25), // Headers
        Constraint::Min(0), // Body
        Constraint::Length(3), // Attachment
    ]).split(editor_root[1]);

    app.url_rect = editor_area[0]; app.headers_rect = editor_area[2]; app.body_rect = editor_area[3]; app.attach_rect = editor_area[4];

    let (input_mode, active_panel) = (app.input_mode, app.active_panel);
    let tab = &mut app.tabs[app.active_tab];

    // URL
    tab.url_area.set_block(Block::default().title(format!(" ⚡ {} URL ", tab.method)).borders(Borders::ALL).border_style(get_editor_border(active_panel, tab.editor_focus, EditorFocus::Url)));
    configure_cursor(tab, input_mode, EditorFocus::Url);
    f.render_widget(tab.url_area.widget(), editor_area[0]);

    // Body Type Selector
    let bt_titles = vec![" JSON ", " TEXT ", " FORM "];
    let bt_idx = match tab.body_type { BodyType::Json => 0, BodyType::Text => 1, BodyType::Form => 2 };
    let bt_tabs = Tabs::new(bt_titles)
        .block(Block::default().title(" ⚙️ BODY TYPE ").borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
        .select(bt_idx)
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
    f.render_widget(bt_tabs, editor_area[1]);

    // Headers
    tab.headers_area.set_block(Block::default().title(" 📋 HEADERS ").borders(Borders::ALL).border_style(get_editor_border(active_panel, tab.editor_focus, EditorFocus::Headers)));
    configure_cursor(tab, input_mode, EditorFocus::Headers);
    f.render_widget(tab.headers_area.widget(), editor_area[2]);

    // Body
    tab.body_area.set_block(Block::default().title(" 📦 BODY ").borders(Borders::ALL).border_style(get_editor_border(active_panel, tab.editor_focus, EditorFocus::Body)));
    configure_cursor(tab, input_mode, EditorFocus::Body);
    f.render_widget(tab.body_area.widget(), editor_area[3]);

    // Attachment
    let att_style = if tab.editor_focus == EditorFocus::Attachment { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Cyan) };
    let att_content = if tab.file_path.is_empty() { "Press ENTER to browse...".to_string() } else { format!("📎 {}", tab.file_path) };
    f.render_widget(Paragraph::new(att_content).style(att_style).block(Block::default().title(" 🖇 ATTACHMENT ").borders(Borders::ALL).border_style(att_style)), editor_area[4]);

    // 3. Response & AI
    let right_chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(main_chunks[2]);
    
    let response_content = if let Some(bytes) = &tab.response_bytes {
        let preview = crate::img_preview::generate_hifi_preview(bytes, right_chunks[0].width.saturating_sub(4) as u32);
        Text::raw(preview)
    } else {
        highlight_json(&tab.response)
    };

    f.render_widget(Paragraph::new(response_content).block(Block::default().title(" 📡 RESPONSE ").borders(Borders::ALL).border_style(get_border_style(active_panel, ActivePanel::Response))).scroll((tab.response_scroll, 0)).wrap(Wrap { trim: false }), right_chunks[0]);
    f.render_widget(Paragraph::new(app.ai_response.as_str()).style(Style::default().fg(Color::Magenta)).block(Block::default().title(" 🧠 AI AGENT ").borders(Borders::ALL).border_style(get_border_style(active_panel, ActivePanel::AI))).wrap(Wrap { trim: true }), right_chunks[1]);

    // Footer
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(75)])
        .split(chunks[1]);

    let footer_text = " [H] Hist | [N] Tab | [F] Foc | [I] Ins | [^P] Curl | [G] Swagger | [O] Open | [K] API ";
    f.render_widget(Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray)).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::Magenta))), footer_chunks[0]);

    // Dashboard de Sistema - ARTHEMA alineado a la derecha
    let sys_metrics = Line::from(vec![
        Span::styled(format!(" ⚡ BAT: {} ", app.battery_level), Style::default().fg(Color::Cyan)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" CPU: {:.1}% ", app.cpu_usage), Style::default().fg(Color::Cyan)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" MEM: {}MB ", app.mem_used), Style::default().fg(Color::Cyan)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(format!(" ARTHEMA v{} : {:.1}% {}MB ", env!("CARGO_PKG_VERSION"), app.proc_cpu, app.proc_mem), Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(sys_metrics).alignment(Alignment::Right).block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::Magenta))), footer_chunks[1]);

    // MODAL: File Picker
    // MODAL: API Key Input
    if app.show_key_input {
        let area = centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        app.key_input.set_block(Block::default().title(" 🔑 CONFIGURE GEMINI API KEY (ENTER to save, ESC to cancel) ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
        app.key_input.set_cursor_style(Style::default().bg(Color::Yellow).fg(Color::Black));
        f.render_widget(app.key_input.widget(), area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2)].as_ref()).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2)].as_ref()).split(popup_layout[1])[1]
}

fn highlight_json(text: &str) -> Text<'_> {
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut spans = Vec::new();
        if line.contains("STATUS:") { spans.push(Span::styled(line, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))); }
        else if line.trim().starts_with('\"') && line.contains(':') {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            spans.push(Span::styled(parts[0], Style::default().fg(Color::LightBlue)));
            spans.push(Span::styled(":", Style::default().fg(Color::White)));
            if parts.len() > 1 { spans.push(Span::styled(parts[1], Style::default().fg(Color::LightYellow))); }
        } else if line.contains('{') || line.contains('}') || line.contains('[') || line.contains(']') {
            spans.push(Span::styled(line, Style::default().fg(Color::Magenta)));
        } else { spans.push(Span::styled(line, Style::default().fg(Color::Gray))); }
        lines.push(Line::from(spans));
    }
    Text::from(lines)
}

fn configure_cursor(tab: &mut crate::app::RequestTab, input_mode: bool, focus: EditorFocus) {
    let area = match focus { EditorFocus::Url => &mut tab.url_area, EditorFocus::Headers => &mut tab.headers_area, EditorFocus::Body => &mut tab.body_area, _ => return };
    if input_mode && tab.editor_focus == focus { area.set_cursor_style(Style::default().bg(Color::Yellow).fg(Color::Black)); }
    else { area.set_cursor_style(Style::default()); }
}

fn get_editor_border(active_panel: ActivePanel, current_focus: EditorFocus, target_focus: EditorFocus) -> Style {
    if matches!(active_panel, ActivePanel::Editor) && current_focus == target_focus { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) }
    else { Style::default().fg(Color::Cyan) }
}

fn get_border_style(active: ActivePanel, current: ActivePanel) -> Style {
    if active == current { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
    else { Style::default().fg(Color::Rgb(60, 60, 60)) }
}

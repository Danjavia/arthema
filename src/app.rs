use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::sync::mpsc;
use crate::collections::{CollectionManager, ApiRequest};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use reqwest::Method;
use tui_textarea::{TextArea, CursorMove};
use std::process::{Command, Stdio};
use std::io::Write;
use ratatui::layout::Rect;
use sysinfo::{System, Pid};

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel {
    Collections,
    Editor,
    Response,
    AI,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EditorFocus {
    Url,
    Headers,
    Body,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LeftPanelTab {
    Collections,
    History,
}

pub struct RequestTab<'a> {
    pub name: String,
    pub url_area: TextArea<'a>,
    pub headers_area: TextArea<'a>,
    pub body_area: TextArea<'a>,
    pub method: String,
    pub response: String,
    pub editor_focus: EditorFocus,
    pub response_scroll: u16,
}

impl<'a> RequestTab<'a> {
    pub fn new(name: String) -> Self {
        let mut url_area = TextArea::default();
        url_area.insert_str("https://jsonplaceholder.typicode.com/posts");
        let mut headers_area = TextArea::default();
        headers_area.insert_str("Content-Type: application/json");
        let mut body_area = TextArea::default();
        body_area.insert_str("{\n  \"title\": \"foo\",\n  \"body\": \"bar\"\n}");

        Self {
            name,
            url_area,
            headers_area,
            body_area,
            method: "GET".to_string(),
            response: "".to_string(),
            editor_focus: EditorFocus::Url,
            response_scroll: 0,
        }
    }
}

pub struct App<'a> {
    pub tabs: Vec<RequestTab<'a>>,
    pub active_tab: usize,
    pub ai_response: String,
    pub active_panel: ActivePanel,
    pub left_panel_tab: LeftPanelTab,
    pub input_mode: bool,
    pub is_ai_loading: bool,
    pub tx: mpsc::Sender<String>,
    pub rx: mpsc::Receiver<String>,
    pub collections: CollectionManager,
    pub selected_idx: usize,
    pub last_click_time: Instant,
    pub url_rect: Rect,
    pub headers_rect: Rect,
    pub body_rect: Rect,
    pub sys: System,
    pub cpu_usage: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub proc_cpu: f32,
    pub proc_mem: u64,
    pub battery_level: String,
    pub last_sys_update: Instant,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let (tx, rx) = mpsc::channel();
        let collections = CollectionManager::new();
        let mut sys = System::new_all();
        sys.refresh_all();

        App {
            tabs: vec![RequestTab::new("Req 1".to_string())],
            active_tab: 0,
            ai_response: "ARTHEMA SYSTEM READY".to_string(),
            active_panel: ActivePanel::Editor,
            left_panel_tab: LeftPanelTab::Collections,
            input_mode: false,
            is_ai_loading: false,
            tx,
            rx,
            collections,
            selected_idx: 0,
            last_click_time: Instant::now(),
            url_rect: Rect::default(),
            headers_rect: Rect::default(),
            body_rect: Rect::default(),
            sys,
            cpu_usage: 0.0,
            mem_total: 0,
            mem_used: 0,
            proc_cpu: 0.0,
            proc_mem: 0,
            battery_level: "N/A".to_string(),
            last_sys_update: Instant::now(),
        }
    }

    pub fn current_tab(&self) -> &RequestTab<'a> { &self.tabs[self.active_tab] }
    pub fn current_tab_mut(&mut self) -> &mut RequestTab<'a> { &mut self.tabs[self.active_tab] }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, _w: u16, _h: u16) {
        let x = mouse.column;
        let y = mouse.row;

        if let MouseEventKind::Down(_) = mouse.kind {
            if self.url_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor; self.current_tab_mut().editor_focus = EditorFocus::Url;
                self.input_mode = true;
                let rx = x.saturating_sub(self.url_rect.x + 1); let ry = y.saturating_sub(self.url_rect.y + 1);
                self.current_tab_mut().url_area.move_cursor(CursorMove::Jump(ry, rx));
            } else if self.headers_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor; self.current_tab_mut().editor_focus = EditorFocus::Headers;
                self.input_mode = true;
                let rx = x.saturating_sub(self.headers_rect.x + 1); let ry = y.saturating_sub(self.headers_rect.y + 1);
                self.current_tab_mut().headers_area.move_cursor(CursorMove::Jump(ry, rx));
            } else if self.body_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor; self.current_tab_mut().editor_focus = EditorFocus::Body;
                self.input_mode = true;
                let rx = x.saturating_sub(self.body_rect.x + 1); let ry = y.saturating_sub(self.body_rect.y + 1);
                self.current_tab_mut().body_area.move_cursor(CursorMove::Jump(ry, rx));
            } else if x < self.url_rect.x {
                self.active_panel = ActivePanel::Collections;
            } else if x > (self.url_rect.x + self.url_rect.width) {
                self.active_panel = ActivePanel::Response;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => { self.copy_to_system(); return; }
                KeyCode::Char('v') => { self.paste_from_pbpaste(); return; }
                KeyCode::Char('z') => { self.undo_active(); return; }
                KeyCode::Char('t') => { self.new_tab(); return; }
                KeyCode::Char('w') => { self.close_tab(); return; }
                _ => {}
            }
        }

        if self.input_mode {
            if key.code == KeyCode::Esc { self.input_mode = false; return; }
            
            // Especial: Enter en URL ejecuta
            if key.code == KeyCode::Enter && self.current_tab().editor_focus == EditorFocus::Url {
                self.input_mode = false;
                self.send_request();
                return;
            }

            let tab = self.current_tab_mut();
            match tab.editor_focus {
                EditorFocus::Url => { tab.url_area.input(key); }
                EditorFocus::Headers => { tab.headers_area.input(key); }
                EditorFocus::Body => { tab.body_area.input(key); }
            }
            return;
        }

        match key.code {
            KeyCode::Char('i') => self.input_mode = true,
            KeyCode::Char('h') => self.toggle_left_panel(),
            KeyCode::Char('m') => self.cycle_method(true),
            KeyCode::Char('M') => self.cycle_method(false),
            KeyCode::Char('f') => self.cycle_editor_focus(),
            KeyCode::Char('a') => self.trigger_ai_suggestion(),
            KeyCode::Char('e') => self.trigger_ai_explain(),
            KeyCode::Char('x') => self.trigger_ai_fix(),
            KeyCode::Char('s') => self.save_current_request(),
            KeyCode::Char('c') => self.copy_to_system(), // Tecla 'c' para copiar
            KeyCode::Char('n') => self.next_tab(),
            KeyCode::Enter => {
                if matches!(self.active_panel, ActivePanel::Collections) { self.load_selected_item(); }
                else { self.send_request(); }
            }
            KeyCode::Tab => self.next_panel(),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            _ => {}
        }
    }

    fn toggle_left_panel(&mut self) {
        self.left_panel_tab = match self.left_panel_tab {
            LeftPanelTab::Collections => LeftPanelTab::History,
            LeftPanelTab::History => LeftPanelTab::Collections,
        };
        self.selected_idx = 0;
    }

    fn new_tab(&mut self) {
        let id = self.tabs.len() + 1;
        self.tabs.push(RequestTab::new(format!("Req {}", id)));
        self.active_tab = self.tabs.len() - 1;
    }

    fn close_tab(&mut self) { if self.tabs.len() > 1 { self.tabs.remove(self.active_tab); self.active_tab = self.active_tab.saturating_sub(1); } }
    fn next_tab(&mut self) { self.active_tab = (self.active_tab + 1) % self.tabs.len(); }

    fn undo_active(&mut self) {
        let tab = self.current_tab_mut();
        match tab.editor_focus {
            EditorFocus::Url => { tab.url_area.undo(); }
            EditorFocus::Headers => { tab.headers_area.undo(); }
            EditorFocus::Body => { tab.body_area.undo(); }
        }
    }

    fn copy_to_system(&mut self) {
        let text = match self.active_panel {
            ActivePanel::Editor => {
                let tab = self.current_tab();
                let area = match tab.editor_focus { EditorFocus::Url => &tab.url_area, EditorFocus::Headers => &tab.headers_area, EditorFocus::Body => &tab.body_area };
                area.lines().join("\n")
            },
            ActivePanel::Response => {
                let res = self.current_tab().response.clone();
                if let Some(pos) = res.find("\n\n") { res[pos+2..].to_string() } else { res }
            },
            ActivePanel::AI => self.ai_response.clone(),
            _ => "".to_string(),
        };
        if text.is_empty() { return; }
        let _ = Command::new("pbcopy").stdin(Stdio::piped()).spawn().and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(text.as_bytes()); }
            let _ = child.wait(); Ok(())
        });
        self.ai_response = format!("SYSTEM: Copied {} chars to clipboard.", text.len());
    }

    fn paste_from_pbpaste(&mut self) {
        if let Ok(output) = Command::new("pbpaste").output() {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            let tab = self.current_tab_mut();
            match tab.editor_focus {
                EditorFocus::Url => { tab.url_area.insert_str(text); }
                EditorFocus::Headers => { tab.headers_area.insert_str(text); }
                EditorFocus::Body => { tab.body_area.insert_str(text); }
            }
            self.input_mode = true;
        }
    }

    fn cycle_editor_focus(&mut self) {
        let tab = self.current_tab_mut();
        tab.editor_focus = match tab.editor_focus {
            EditorFocus::Url => EditorFocus::Headers,
            EditorFocus::Headers => EditorFocus::Body,
            EditorFocus::Body => EditorFocus::Url,
        };
    }

    fn cycle_method(&mut self, forward: bool) {
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        let current = self.current_tab().method.clone();
        let pos = methods.iter().position(|&m| m == current).unwrap_or(0) as i32;
        let next = if forward { (pos + 1).rem_euclid(methods.len() as i32) }
                   else { (pos - 1).rem_euclid(methods.len() as i32) };
        self.current_tab_mut().method = methods[next as usize].to_string();
    }

    fn move_selection(&mut self, delta: i32) {
        match self.active_panel {
            ActivePanel::Collections => {
                let count = match self.left_panel_tab {
                    LeftPanelTab::Collections => self.collections.requests.len(),
                    LeftPanelTab::History => self.collections.history.len(),
                };
                if count > 0 { self.selected_idx = (self.selected_idx as i32 + delta).rem_euclid(count as i32) as usize; }
            }
            ActivePanel::Response => {
                let tab = self.current_tab_mut();
                if delta > 0 { tab.response_scroll = tab.response_scroll.saturating_add(1); }
                else { tab.response_scroll = tab.response_scroll.saturating_sub(1); }
            }
            _ => {}
        }
    }

    fn load_selected_item(&mut self) {
        let req_clone = match self.left_panel_tab {
            LeftPanelTab::Collections => self.collections.requests.get(self.selected_idx).cloned(),
            LeftPanelTab::History => self.collections.history.get(self.selected_idx).cloned(),
        };
        if let Some(req) = req_clone {
            let tab = self.current_tab_mut();
            tab.url_area = TextArea::default(); tab.url_area.insert_str(&req.url);
            tab.headers_area = TextArea::default(); 
            let h_str = req.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n");
            tab.headers_area.insert_str(h_str);
            tab.body_area = TextArea::default(); if let Some(b) = &req.body { tab.body_area.insert_str(b); }
            tab.method = req.method.clone();
            self.active_panel = ActivePanel::Editor;
        }
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Collections => ActivePanel::Editor,
            ActivePanel::Editor => ActivePanel::Response,
            ActivePanel::Response => ActivePanel::AI,
            ActivePanel::AI => ActivePanel::Collections,
        };
    }

    fn save_current_request(&mut self) {
        let (name, url, method, h_lines, body) = {
            let tab = self.current_tab();
            (tab.name.clone(), tab.url_area.lines()[0].clone(), tab.method.clone(), tab.headers_area.lines().iter().map(|s| s.to_string()).collect::<Vec<_>>(), tab.body_area.lines().join("\n"))
        };
        let mut headers = HashMap::new();
        for line in h_lines {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 { headers.insert(parts[0].trim().to_string(), parts[1].trim().to_string()); }
        }
        let new_req = ApiRequest { name: format!("Req_{}", self.collections.requests.len() + 1), url, method, headers, body: Some(body) };
        if self.collections.save_request(&new_req).is_ok() { let _ = self.collections.load_all(); self.ai_response = "SYSTEM: archived.".to_string(); }
    }

    pub fn send_request(&mut self) {
        let tx = self.tx.clone();
        let (url, method_str, body_content, h_lines) = {
            let tab = self.current_tab_mut();
            tab.response = "SYNCING...".to_string();
            tab.response_scroll = 0;
            (tab.url_area.lines()[0].clone(), tab.method.clone(), tab.body_area.lines().join("\n"), tab.headers_area.lines().iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };
        let mut h_map = HashMap::new();
        for line in h_lines {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 { h_map.insert(parts[0].trim().to_string(), parts[1].trim().to_string()); }
        }
        let hist_req = ApiRequest { name: url.clone(), url: url.clone(), method: method_str.clone(), headers: h_map.clone(), body: Some(body_content.clone()) };
        self.collections.add_to_history(hist_req);

        tokio::spawn(async move {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap();
            let method = match method_str.as_str() { "POST" => Method::POST, "PUT" => Method::PUT, "DELETE" => Method::DELETE, "PATCH" => Method::PATCH, "HEAD" => Method::HEAD, "OPTIONS" => Method::OPTIONS, _ => Method::GET };
            let mut rb = client.request(method.clone(), &url);
            for (k, v) in h_map { rb = rb.header(k, v); }
            if !body_content.is_empty() && method != Method::GET && method != Method::HEAD { rb = rb.body(body_content); }
            match rb.header("User-Agent", "Arthema").send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_else(|_| "ERR".to_string());
                    let formatted = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) { serde_json::to_string_pretty(&val).unwrap_or(text) } else { text };
                    let _ = tx.send(format!("STATUS: {}\n\n{}", status, formatted));
                }
                Err(e) => { let _ = tx.send(format!("ERROR: {}", e)); }
            }
        });
    }

    pub fn trigger_ai_suggestion(&mut self) {
        if self.is_ai_loading { return; }
        self.is_ai_loading = true;
        let tx = self.tx.clone();
        let url = self.current_tab().url_area.lines()[0].clone();
        self.ai_response = "AI: CALCULATING...".to_string();
        tokio::spawn(async move {
            let s = crate::ai::get_ai_suggestion(&url).await;
            let _ = tx.send(format!("AI_SUGGESTION:{}", s));
        });
    }

    pub fn trigger_ai_explain(&mut self) {
        if self.is_ai_loading { return; }
        self.is_ai_loading = true;
        let tx = self.tx.clone();
        let r = self.current_tab().response.clone();
        self.ai_response = "AI: ANALYZING...".to_string();
        tokio::spawn(async move {
            let e = crate::ai::explain_response(&r).await;
            let _ = tx.send(format!("AI_EXPLANATION:{}", e));
        });
    }

    pub fn trigger_ai_fix(&mut self) {
        if self.is_ai_loading { return; }
        self.is_ai_loading = true;
        let tab = self.current_tab();
        let tx = self.tx.clone();
        let (m, u, h, b, e) = (tab.method.clone(), tab.url_area.lines()[0].clone(), tab.headers_area.lines().join("\n"), tab.body_area.lines().join("\n"), tab.response.clone());
        self.ai_response = "AI FIXER: DIAGNOSING...".to_string();
        tokio::spawn(async move {
            let e = crate::ai::fix_error(&m, &u, &h, &b, &e).await;
            let _ = tx.send(format!("AI_EXPLANATION:{}", e));
        });
    }

    pub fn update(&mut self) {
        while let Ok(res) = self.rx.try_recv() {
            self.is_ai_loading = false;
            if let Some(s) = res.strip_prefix("AI_SUGGESTION:") {
                let tab = self.current_tab_mut();
                tab.url_area = TextArea::default(); tab.url_area.insert_str(s.trim());
                self.ai_response = "AI: Suggested route loaded.".to_string();
            } else if let Some(e) = res.strip_prefix("AI_EXPLANATION:") {
                self.ai_response = e.to_string();
            } else { self.current_tab_mut().response = res; }
        }

        if self.last_sys_update.elapsed() > Duration::from_secs(2) {
            self.sys.refresh_cpu(); self.sys.refresh_memory();
            self.cpu_usage = self.sys.global_cpu_info().cpu_usage();
            self.mem_total = self.sys.total_memory() / 1024 / 1024;
            self.mem_used = self.sys.used_memory() / 1024 / 1024;
            let pid = Pid::from_u32(std::process::id());
            self.sys.refresh_process(pid);
            if let Some(proc) = self.sys.process(pid) {
                let num_cpus = self.sys.cpus().len() as f32;
                self.proc_cpu = proc.cpu_usage() / num_cpus;
                self.proc_mem = proc.memory() / 1024 / 1024;
            }
            if let Ok(output) = Command::new("pmset").arg("-g").arg("batt").output() {
                let out = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = out.lines().nth(1) {
                    if let Some(perc) = line.split('\t').nth(1) {
                        self.battery_level = perc.split(';').next().unwrap_or("N/A").to_string();
                    }
                }
            }
            self.last_sys_update = Instant::now();
        }
    }

    fn select_all_active(&mut self) {
        let tab = self.current_tab_mut();
        let area = match tab.editor_focus { EditorFocus::Url => &mut tab.url_area, EditorFocus::Headers => &mut tab.headers_area, EditorFocus::Body => &mut tab.body_area };
        area.move_cursor(tui_textarea::CursorMove::Top); area.start_selection(); area.move_cursor(tui_textarea::CursorMove::Bottom); area.move_cursor(tui_textarea::CursorMove::End);
    }
}

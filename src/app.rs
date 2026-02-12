use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::sync::mpsc;
use crate::collections::{CollectionManager, ApiRequest};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use reqwest::Method;
use tui_textarea::{TextArea, CursorMove};
use std::process::{Command, Stdio};
use std::io::Write;
use std::fs;
use ratatui::layout::Rect;

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

pub struct App<'a> {
    pub url_area: TextArea<'a>,
    pub headers_area: TextArea<'a>,
    pub body_area: TextArea<'a>,
    pub method: String,
    pub response: String,
    pub ai_response: String,
    pub active_panel: ActivePanel,
    pub editor_focus: EditorFocus,
    pub input_mode: bool,
    pub tx: mpsc::Sender<String>,
    pub rx: mpsc::Receiver<String>,
    pub collections: CollectionManager,
    pub selected_collection_idx: usize,
    pub response_scroll: u16,
    pub last_click_time: Instant,
    pub url_rect: Rect,
    pub headers_rect: Rect,
    pub body_rect: Rect,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let (tx, rx) = mpsc::channel();
        let collections = CollectionManager::new();
        let mut url_area = TextArea::default();
        url_area.insert_str("https://jsonplaceholder.typicode.com/posts");
        let mut headers_area = TextArea::default();
        headers_area.insert_str("Content-Type: application/json");
        let mut body_area = TextArea::default();
        body_area.insert_str("{\n  \"title\": \"foo\",\n  \"body\": \"bar\",\n  \"userId\": 1\n}");

        App {
            url_area,
            headers_area,
            body_area,
            method: "GET".to_string(),
            response: "".to_string(),
            ai_response: "ARTHEMA SYSTEM READY".to_string(),
            active_panel: ActivePanel::Editor,
            editor_focus: EditorFocus::Url,
            input_mode: false,
            tx,
            rx,
            collections,
            selected_collection_idx: 0,
            response_scroll: 0,
            last_click_time: Instant::now(),
            url_rect: Rect::default(),
            headers_rect: Rect::default(),
            body_rect: Rect::default(),
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, _w: u16, _h: u16) {
        let x = mouse.column;
        let y = mouse.row;

        if let MouseEventKind::Down(_) = mouse.kind {
            let now = Instant::now();
            let is_double = now.duration_since(self.last_click_time) < Duration::from_millis(400);
            self.last_click_time = now;

            // 1. Detección de Foco y Posicionamiento Forzado
            if self.url_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor;
                self.editor_focus = EditorFocus::Url;
                self.input_mode = true;
                let rel_x = x.saturating_sub(self.url_rect.x + 1);
                let rel_y = y.saturating_sub(self.url_rect.y + 1);
                self.url_area.move_cursor(CursorMove::Jump(rel_y, rel_x));
            } else if self.headers_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor;
                self.editor_focus = EditorFocus::Headers;
                self.input_mode = true;
                let rel_x = x.saturating_sub(self.headers_rect.x + 1);
                let rel_y = y.saturating_sub(self.headers_rect.y + 1);
                self.headers_area.move_cursor(CursorMove::Jump(rel_y, rel_x));
            } else if self.body_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor;
                self.editor_focus = EditorFocus::Body;
                self.input_mode = true;
                let rel_x = x.saturating_sub(self.body_rect.x + 1);
                let rel_y = y.saturating_sub(self.body_rect.y + 1);
                self.body_area.move_cursor(CursorMove::Jump(rel_y, rel_x));
            } else if x < self.url_rect.x {
                self.active_panel = ActivePanel::Collections;
            } else if x > (self.url_rect.x + self.url_rect.width) {
                self.active_panel = ActivePanel::AI;
            }
            
            if is_double && matches!(self.active_panel, ActivePanel::Editor) {
                self.select_all_active();
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => { self.copy_to_system(); return; }
                KeyCode::Char('v') => { self.paste_from_pbpaste(); return; }
                KeyCode::Char('z') => { self.undo_active(); return; }
                KeyCode::Char('a') => { self.select_all_active(); return; }
                _ => {}
            }
        }

        if self.input_mode {
            if key.code == KeyCode::Esc { self.input_mode = false; return; }
            match self.editor_focus {
                EditorFocus::Url => { self.url_area.input(key); }
                EditorFocus::Headers => { self.headers_area.input(key); }
                EditorFocus::Body => { self.body_area.input(key); }
            }
            return;
        }

        match key.code {
            KeyCode::Char('i') => self.input_mode = true,
            KeyCode::Char('c') => self.copy_to_system(),
            KeyCode::Char('m') => self.cycle_method(),
            KeyCode::Char('f') => self.cycle_editor_focus(),
            KeyCode::Char('a') => self.trigger_ai_suggestion(),
            KeyCode::Char('e') => self.trigger_ai_explain(),
            KeyCode::Char('s') => self.save_current_request(),
            KeyCode::Enter => {
                if matches!(self.active_panel, ActivePanel::Collections) {
                    self.load_selected_collection();
                } else {
                    self.send_request();
                }
            }
            KeyCode::Tab => self.next_panel(),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            _ => {}
        }
    }

    fn select_all_active(&mut self) {
        let area = match self.editor_focus {
            EditorFocus::Url => &mut self.url_area,
            EditorFocus::Headers => &mut self.headers_area,
            EditorFocus::Body => &mut self.body_area,
        };
        area.move_cursor(tui_textarea::CursorMove::Top);
        area.start_selection();
        area.move_cursor(tui_textarea::CursorMove::Bottom);
        area.move_cursor(tui_textarea::CursorMove::End);
    }

    fn undo_active(&mut self) {
        match self.editor_focus {
            EditorFocus::Url => { self.url_area.undo(); }
            EditorFocus::Headers => { self.headers_area.undo(); }
            EditorFocus::Body => { self.body_area.undo(); }
        }
    }

    fn copy_to_system(&mut self) {
        let text = match self.active_panel {
            ActivePanel::Editor => {
                let area = match self.editor_focus {
                    EditorFocus::Url => &self.url_area,
                    EditorFocus::Headers => &self.headers_area,
                    EditorFocus::Body => &self.body_area,
                };
                area.lines().join("\n")
            },
            ActivePanel::Response => self.response.clone(),
            _ => "".to_string(),
        };
        if text.is_empty() { return; }
        if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(text.as_bytes()); }
            let _ = child.wait();
            self.ai_response = format!("SYSTEM: Copied {} chars.", text.len());
        }
    }

    fn paste_from_pbpaste(&mut self) {
        if let Ok(output) = Command::new("pbpaste").output() {
            let text = String::from_utf8_lossy(&output.stdout);
            match self.editor_focus {
                EditorFocus::Url => { self.url_area.insert_str(text); }
                EditorFocus::Headers => { self.headers_area.insert_str(text); }
                EditorFocus::Body => { self.body_area.insert_str(text); }
            }
            self.input_mode = true;
        }
    }

    fn cycle_editor_focus(&mut self) {
        self.editor_focus = match self.editor_focus {
            EditorFocus::Url => EditorFocus::Headers,
            EditorFocus::Headers => EditorFocus::Body,
            EditorFocus::Body => EditorFocus::Url,
        };
    }

    fn cycle_method(&mut self) {
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
        let current = self.method.clone();
        let current_idx = methods.iter().position(|&m| m == current).unwrap_or(0);
        self.method = methods[(current_idx + 1) % methods.len()].to_string();
    }

    fn move_selection(&mut self, delta: i32) {
        match self.active_panel {
            ActivePanel::Collections => {
                let count = self.collections.requests.len();
                if count > 0 {
                    let new_idx = (self.selected_collection_idx as i32 + delta).rem_euclid(count as i32);
                    self.selected_collection_idx = new_idx as usize;
                }
            }
            ActivePanel::Response => {
                if delta > 0 { self.response_scroll += 1; }
                else if self.response_scroll > 0 { self.response_scroll -= 1; }
            }
            _ => {}
        }
    }

    fn load_selected_collection(&mut self) {
        if let Some(req) = self.collections.requests.get(self.selected_collection_idx) {
            self.url_area = TextArea::default();
            self.url_area.insert_str(&req.url);
            self.headers_area = TextArea::default();
            let h_str = req.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n");
            self.headers_area.insert_str(h_str);
            self.body_area = TextArea::default();
            if let Some(b) = &req.body { self.body_area.insert_str(b); }
            self.method = req.method.clone();
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
        let mut headers = HashMap::new();
        for line in self.headers_area.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 { headers.insert(parts[0].trim().to_string(), parts[1].trim().to_string()); }
        }
        let new_req = ApiRequest {
            name: format!("Req_{}", self.collections.requests.len() + 1),
            url: self.url_area.lines()[0].clone(),
            method: self.method.clone(),
            headers,
            body: Some(self.body_area.lines().join("\n")),
        };
        if self.collections.save_request(&new_req).is_ok() {
            let _ = self.collections.load_all();
            self.ai_response = "SYSTEM: archived.".to_string();
        }
    }

    pub fn send_request(&mut self) {
        self.response = "SYNCING...".to_string();
        let tx = self.tx.clone();
        let url = self.url_area.lines()[0].clone();
        let method_str = self.method.clone();
        let body_content = self.body_area.lines().join("\n");
        let mut h_map = HashMap::new();
        for line in self.headers_area.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 { h_map.insert(parts[0].trim().to_string(), parts[1].trim().to_string()); }
        }
        tokio::spawn(async move {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap();
            let method = match method_str.as_str() {
                "POST" => Method::POST, "PUT" => Method::PUT, "DELETE" => Method::DELETE,
                "PATCH" => Method::PATCH, "HEAD" => Method::HEAD, "OPTIONS" => Method::OPTIONS,
                _ => Method::GET,
            };
            let mut rb = client.request(method.clone(), &url);
            for (k, v) in h_map { rb = rb.header(k, v); }
            if !body_content.is_empty() && method != Method::GET && method != Method::HEAD { rb = rb.body(body_content); }
            match rb.header("User-Agent", "Arthema").send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_else(|_| "ERR".to_string());
                    let formatted = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        serde_json::to_string_pretty(&val).unwrap_or(text)
                    } else { text };
                    let _ = tx.send(format!("STATUS: {}\n\n{}", status, formatted));
                }
                Err(e) => { let _ = tx.send(format!("ERROR: {}", e)); }
            }
        });
    }

    pub fn trigger_ai_suggestion(&mut self) {
        let tx = self.tx.clone();
        let url = self.url_area.lines()[0].clone();
        self.ai_response = "AI: CALCULATING...".to_string();
        tokio::spawn(async move {
            let s = crate::ai::get_ai_suggestion(&url).await;
            let _ = tx.send(format!("AI_SUGGESTION:{}", s));
        });
    }

    pub fn trigger_ai_explain(&mut self) {
        let tx = self.tx.clone();
        let r = self.response.clone();
        self.ai_response = "AI: ANALYZING...".to_string();
        tokio::spawn(async move {
            let e = crate::ai::explain_response(&r).await;
            let _ = tx.send(format!("AI_EXPLANATION:{}", e));
        });
    }

    pub fn update(&mut self) {
        while let Ok(res) = self.rx.try_recv() {
            if let Some(s) = res.strip_prefix("AI_SUGGESTION:") {
                self.url_area = TextArea::default();
                self.url_area.insert_str(s.trim());
                self.ai_response = "AI: Suggested route loaded.".to_string();
            } else if let Some(e) = res.strip_prefix("AI_EXPLANATION:") {
                self.ai_response = e.to_string();
            } else { self.response = res; }
        }
    }
}

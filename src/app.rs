use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::sync::mpsc;
use crate::collections::{CollectionManager, ApiRequest};
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use reqwest::Method;
use tui_textarea::{TextArea, CursorMove};
use std::process::{Command, Stdio};
use std::io::Write;
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use sysinfo::{System, Pid};
use std::fs;
use std::path::{PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum ActivePanel { Collections, Editor, Response, AI }

#[derive(Clone, Copy, PartialEq)]
pub enum EditorFocus { Url, Headers, Body, Attachment }

#[derive(Clone, Copy, PartialEq)]
pub enum BodyType { Json, Text, Form }

#[derive(Clone, Copy, PartialEq)]
pub enum LeftPanelTab { Collections, History }

pub enum ResponseData {
    Text(String),
    Binary(String, Vec<u8>), // Texto descriptivo + bytes
}

pub struct RequestTab<'a> {
    pub name: String,
    pub url_area: TextArea<'a>,
    pub headers_area: TextArea<'a>,
    pub body_area: TextArea<'a>,
    pub file_path: String,
    pub method: String,
    pub response: String,
    pub response_bytes: Option<Vec<u8>>,
    pub editor_focus: EditorFocus,
    pub body_type: BodyType,
    pub response_scroll: u16,
    pub is_tree_mode: bool,
}

impl<'a> RequestTab<'a> {
    pub fn new(name: String) -> Self {
        let mut url_area = TextArea::default(); url_area.insert_str("https://jsonplaceholder.typicode.com/posts");
        let mut headers_area = TextArea::default(); headers_area.insert_str("Content-Type: application/json");
        let mut body_area = TextArea::default(); body_area.insert_str("{\n  \"title\": \"Arthema Request\"\n}");
        Self {
            name, url_area, headers_area, body_area,
            file_path: "".to_string(), method: "GET".to_string(),
            response: "".to_string(), response_bytes: None, editor_focus: EditorFocus::Url,
            body_type: BodyType::Json, response_scroll: 0, is_tree_mode: false,
        }
    }
}

pub enum AppEvent {
    ApiResponse(String, Option<Vec<u8>>),
    AiMessage(String),
    SystemMessage(String),
    SwaggerImported(Vec<ApiRequest>),
}

pub enum CollectionItem {
    Folder(String),
    Request(usize),
}

pub struct App<'a> {
    pub tabs: Vec<RequestTab<'a>>,
    pub active_tab: usize,
    pub ai_response: String,
    pub active_panel: ActivePanel,
    pub left_panel_tab: LeftPanelTab,
    pub expanded_groups: HashSet<String>,
    pub input_mode: bool,
    pub is_ai_loading: bool,
    pub tx: mpsc::Sender<AppEvent>,
    pub rx: mpsc::Receiver<AppEvent>,
    pub collections: CollectionManager,
    pub config: crate::config::Config,
    pub key_input: TextArea<'a>,
    pub show_key_input: bool,
    pub swagger_input: TextArea<'a>,
    pub show_swagger_input: bool,
    pub rename_input: TextArea<'a>,
    pub show_rename_input: bool,
    pub selected_idx: usize,
    pub url_rect: Rect, pub headers_rect: Rect, pub body_rect: Rect, pub attach_rect: Rect,
    pub show_file_picker: bool,
    pub current_dir: PathBuf,
    pub file_entries: Vec<String>,
    pub file_picker_state: ListState,
    pub sys: System, pub cpu_usage: f32, pub mem_total: u64, pub mem_used: u64,
    pub proc_cpu: f32, pub proc_mem: u64, pub battery_level: String, pub last_sys_update: Instant,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let (tx, rx) = mpsc::channel();
        let mut sys = System::new_all(); sys.refresh_all();
        App {
            tabs: vec![RequestTab::new("Req 1".to_string())], active_tab: 0,
            ai_response: "ARTHEMA SYSTEM READY".to_string(),
            active_panel: ActivePanel::Editor, left_panel_tab: LeftPanelTab::Collections,
            expanded_groups: HashSet::new(),
            input_mode: false, is_ai_loading: false, tx, rx, collections: CollectionManager::new(),
            config: crate::config::Config::load(),
            key_input: TextArea::default(),
            show_key_input: false,
            swagger_input: TextArea::default(),
            show_swagger_input: false,
            rename_input: TextArea::default(),
            show_rename_input: false,
            selected_idx: 0,
            url_rect: Rect::default(), headers_rect: Rect::default(), body_rect: Rect::default(), attach_rect: Rect::default(),
            show_file_picker: false, current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")), file_entries: Vec::new(), file_picker_state: ListState::default(),
            sys, cpu_usage: 0.0, mem_total: 0, mem_used: 0, proc_cpu: 0.0, proc_mem: 0,
            battery_level: "N/A".to_string(), last_sys_update: Instant::now(),
        }
    }

    pub fn current_tab(&self) -> &RequestTab<'a> { &self.tabs[self.active_tab] }
    pub fn current_tab_mut(&mut self) -> &mut RequestTab<'a> { &mut self.tabs[self.active_tab] }

    pub fn is_input_active(&self) -> bool {
        self.input_mode || self.show_rename_input || self.show_swagger_input || self.show_key_input || self.show_file_picker
    }

    pub fn get_visible_items(&self) -> Vec<CollectionItem> {
        let mut items = Vec::new();
        let mut groups: Vec<String> = self.collections.requests.iter()
            .filter_map(|r| r.group.clone())
            .collect::<HashSet<_>>().into_iter().collect();
        groups.sort();
        
        // Agregar "UNGROUPED" si hay peticiones sin grupo
        if self.collections.requests.iter().any(|r| r.group.is_none()) {
            groups.push("UNGROUPED".to_string());
        }

        for group in groups {
            items.push(CollectionItem::Folder(group.clone()));
            if self.expanded_groups.contains(&group) {
                for (idx, req) in self.collections.requests.iter().enumerate() {
                    let req_group = req.group.as_deref().unwrap_or("UNGROUPED");
                    if req_group == group {
                        items.push(CollectionItem::Request(idx));
                    }
                }
            }
        }
        items
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, _w: u16, _h: u16) {
        let (x, y) = (mouse.column, mouse.row);
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
            } else if self.attach_rect.contains(ratatui::layout::Position { x, y }) {
                self.active_panel = ActivePanel::Editor; self.current_tab_mut().editor_focus = EditorFocus::Attachment;
                self.open_file_picker();
            } else if x < self.url_rect.x { self.active_panel = ActivePanel::Collections; }
            else if x > (self.url_rect.x + self.url_rect.width) { self.active_panel = ActivePanel::Response; }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.show_rename_input {
            match key.code {
                KeyCode::Esc => { self.show_rename_input = false; }
                KeyCode::Enter => {
                    let new_name = self.rename_input.lines()[0].trim().to_string();
                    if !new_name.is_empty() && matches!(self.left_panel_tab, LeftPanelTab::Collections) {
                        let visible = self.get_visible_items();
                        if let Some(CollectionItem::Request(real_idx)) = visible.get(self.selected_idx) {
                            if let Some(req) = self.collections.requests.get(*real_idx).cloned() {
                                let _ = self.collections.delete_request(*real_idx);
                                let mut updated_req = req;
                                updated_req.name = new_name;
                                let _ = self.collections.save_request(&updated_req);
                                let _ = self.collections.load_all();
                                self.ai_response = "SYSTEM: Request renamed.".to_string();
                            }
                        }
                    }
                    self.show_rename_input = false;
                }
                _ => { self.rename_input.input(key); }
            }
            return; // Bloqueo total de comandos globales
        }
        if self.show_swagger_input {
            match key.code {
                KeyCode::Esc => { self.show_swagger_input = false; }
                KeyCode::Enter => self.import_swagger(),
                _ => { self.swagger_input.input(key); }
            }
            return; // Bloqueo total de comandos globales
        }
        if self.show_key_input {
            match key.code {
                KeyCode::Esc => { self.show_key_input = false; }
                KeyCode::Enter => {
                    let key_str = self.key_input.lines()[0].trim().to_string();
                    if !key_str.is_empty() {
                        self.config.gemini_api_key = Some(key_str);
                        let _ = self.config.save();
                        self.ai_response = "SYSTEM: Gemini API Key updated.".to_string();
                    }
                    self.show_key_input = false;
                }
                _ => { self.key_input.input(key); }
            }
            return; // Bloqueo total de comandos globales
        }
        if self.show_file_picker {
            match key.code {
                KeyCode::Up => { let i = match self.file_picker_state.selected() { Some(i) => if i > 0 { i - 1 } else { self.file_entries.len() - 1 }, None => 0 }; self.file_picker_state.select(Some(i)); }
                KeyCode::Down => { let i = match self.file_picker_state.selected() { Some(i) => if i < self.file_entries.len() - 1 { i + 1 } else { 0 }, None => 0 }; self.file_picker_state.select(Some(i)); }
                KeyCode::Enter => {
                    if let Some(i) = self.file_picker_state.selected() {
                        let entry = self.file_entries[i].clone();
                        if entry == ".." { self.current_dir.pop(); self.refresh_file_entries(); }
                        else {
                            let path = self.current_dir.join(&entry);
                            if path.is_dir() { self.current_dir = path; self.refresh_file_entries(); }
                            else { self.current_tab_mut().file_path = path.to_string_lossy().to_string(); self.show_file_picker = false; }
                        }
                    }
                }
                KeyCode::Esc => self.show_file_picker = false,
                _ => {}
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => { self.copy_to_system(); return; }
                KeyCode::Char('v') => { self.paste_from_pbpaste(); return; }
                KeyCode::Char('p') => { self.import_curl(); return; }
                KeyCode::Char('z') => { self.undo_active(); return; }
                KeyCode::Char('t') => { self.new_tab(); return; }
                KeyCode::Char('w') => { self.handle_delete(); return; } // Ctrl+W también borra pestaña
                _ => {}
            }
        }
        if self.input_mode {
            if key.code == KeyCode::Esc { self.input_mode = false; return; }
            if key.code == KeyCode::Enter && self.current_tab().editor_focus == EditorFocus::Url { self.input_mode = false; self.send_request(); return; }
            let tab = self.current_tab_mut();
            match tab.editor_focus {
                EditorFocus::Url => { tab.url_area.input(key); }
                EditorFocus::Headers => { tab.headers_area.input(key); }
                EditorFocus::Body => { tab.body_area.input(key); }
                _ => {}
            }
            return; // BLOQUEO DEFINITIVO: Si estamos en modo input, no se procesa nada más
        }
        match key.code {
            KeyCode::Char('i') => self.input_mode = true,
            KeyCode::Char('h') => self.toggle_left_panel(),
            KeyCode::Char('d') => self.handle_delete(),
            KeyCode::Char('b') => self.cycle_body_type(),
            KeyCode::Char('t') => { let t = self.current_tab_mut(); t.is_tree_mode = !t.is_tree_mode; }
            KeyCode::Char('m') => self.cycle_method(true),
            KeyCode::Char('M') => self.cycle_method(false),
            KeyCode::Char('f') => self.cycle_editor_focus(),
            KeyCode::Char('s') => self.save_current_request(),
            KeyCode::Char('r') => {
                if matches!(self.left_panel_tab, LeftPanelTab::Collections) {
                    let visible = self.get_visible_items();
                    if let Some(CollectionItem::Request(real_idx)) = visible.get(self.selected_idx) {
                        if let Some(req) = self.collections.requests.get(*real_idx) {
                            self.input_mode = false;
                            self.rename_input = TextArea::default();
                            self.rename_input.insert_str(&req.name);
                            self.show_rename_input = true;
                            self.active_panel = ActivePanel::Collections;
                        }
                    }
                }
            },
            KeyCode::Char('n') => self.next_tab(),
            KeyCode::Char('o') => self.open_in_system(),
            KeyCode::Char('c') => self.copy_to_system(),
            KeyCode::Char('k') => { 
                self.input_mode = false;
                self.show_key_input = true; 
                self.key_input = TextArea::default();
                if let Some(key) = &self.config.gemini_api_key { self.key_input.insert_str(key); }
            },
            KeyCode::Char('g') => {
                self.input_mode = false;
                self.show_swagger_input = true;
                self.swagger_input = TextArea::default();
                self.swagger_input.insert_str("https://petstore.swagger.io/v2/swagger.json");
            },
            KeyCode::Char('a') => self.trigger_ai_suggestion(),
            KeyCode::Char('e') => self.trigger_ai_explain(),
            KeyCode::Char('x') => self.trigger_ai_fix(),
            KeyCode::Enter => { 
                if matches!(self.active_panel, ActivePanel::Collections) { 
                    self.load_selected_item(); 
                } else if matches!(self.current_tab().editor_focus, EditorFocus::Attachment) { 
                    self.open_file_picker(); 
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

    fn handle_delete(&mut self) {
        match self.active_panel {
            ActivePanel::Collections => {
                match self.left_panel_tab {
                    LeftPanelTab::Collections => {
                        let visible = self.get_visible_items();
                        if let Some(CollectionItem::Request(real_idx)) = visible.get(self.selected_idx) {
                            let _ = self.collections.delete_request(*real_idx);
                        }
                    }
                    LeftPanelTab::History => { self.collections.delete_history_item(self.selected_idx); }
                }
                if self.selected_idx > 0 { self.selected_idx -= 1; }
                self.ai_response = "SYSTEM: Item deleted.".to_string();
            },
            ActivePanel::Editor => {
                // Si estamos enfocando el attachment y tiene algo, lo borramos
                if self.current_tab().editor_focus == EditorFocus::Attachment && !self.current_tab().file_path.is_empty() {
                    self.current_tab_mut().file_path.clear();
                    self.ai_response = "SYSTEM: Attachment cleared.".to_string();
                } else {
                    // Si no, borramos la pestaña actual
                    if self.tabs.len() > 1 {
                        self.tabs.remove(self.active_tab);
                        self.active_tab = self.active_tab.saturating_sub(1);
                        self.ai_response = "SYSTEM: Tab closed.".to_string();
                    } else {
                        self.ai_response = "SYSTEM: Cannot close the last tab.".to_string();
                    }
                }
            },
            _ => {}
        }
    }

    fn open_file_picker(&mut self) { self.show_file_picker = true; self.refresh_file_entries(); }
    fn refresh_file_entries(&mut self) {
        self.file_entries.clear(); self.file_entries.push("..".to_string());
        if let Ok(entries) = fs::read_dir(&self.current_dir) { for entry in entries.flatten() { self.file_entries.push(entry.file_name().to_string_lossy().to_string()); } }
        self.file_picker_state.select(Some(0));
    }

    fn cycle_body_type(&mut self) {
        let t = self.current_tab_mut();
        t.body_type = match t.body_type { BodyType::Json => BodyType::Text, BodyType::Text => BodyType::Form, BodyType::Form => BodyType::Json };
        match t.body_type {
            BodyType::Json => { t.headers_area = TextArea::default(); t.headers_area.insert_str("Content-Type: application/json"); }
            BodyType::Text => { t.headers_area = TextArea::default(); t.headers_area.insert_str("Content-Type: text/plain"); }
            BodyType::Form => { t.headers_area = TextArea::default(); t.headers_area.insert_str("Content-Type: multipart/form-data"); }
        }
    }

    fn toggle_left_panel(&mut self) { self.left_panel_tab = match self.left_panel_tab { LeftPanelTab::Collections => LeftPanelTab::History, LeftPanelTab::History => LeftPanelTab::Collections }; self.selected_idx = 0; self.active_panel = ActivePanel::Collections; }
    fn new_tab(&mut self) { self.tabs.push(RequestTab::new(format!("Req {}", self.tabs.len() + 1))); self.active_tab = self.tabs.len() - 1; }
    fn next_tab(&mut self) { self.active_tab = (self.active_tab + 1) % self.tabs.len(); }

    fn undo_active(&mut self) {
        let tab = self.current_tab_mut();
        match tab.editor_focus { EditorFocus::Url => { tab.url_area.undo(); }, EditorFocus::Headers => { tab.headers_area.undo(); }, EditorFocus::Body => { tab.body_area.undo(); }, _ => {} }
    }

    fn copy_to_system(&mut self) {
        let text = match self.active_panel {
            ActivePanel::Editor => { let tab = self.current_tab(); match tab.editor_focus { EditorFocus::Url => tab.url_area.lines().join("\n"), EditorFocus::Headers => tab.headers_area.lines().join("\n"), EditorFocus::Body => tab.body_area.lines().join("\n"), EditorFocus::Attachment => tab.file_path.clone() } },
            ActivePanel::Response => { let r = self.current_tab().response.clone(); if let Some(p) = r.find("\n\n") { r[p+2..].to_string() } else { r } },
            ActivePanel::AI => self.ai_response.clone(),
            _ => "".to_string(),
        };
        if text.is_empty() { return; }
        let _ = Command::new("pbcopy").stdin(Stdio::piped()).spawn().and_then(|mut c| { if let Some(mut s) = c.stdin.take() { let _ = s.write_all(text.as_bytes()); } let _ = c.wait(); Ok(()) });
        self.ai_response = "SYSTEM: Copied.".to_string();
    }

    fn paste_from_pbpaste(&mut self) {
        if let Ok(o) = Command::new("pbpaste").output() {
            let t = String::from_utf8_lossy(&o.stdout).to_string();
            let tab = self.current_tab_mut();
            match tab.editor_focus { EditorFocus::Url => { tab.url_area.insert_str(t); }, EditorFocus::Headers => { tab.headers_area.insert_str(t); }, EditorFocus::Body => { tab.body_area.insert_str(t); }, EditorFocus::Attachment => tab.file_path = t }
            self.input_mode = true;
        }
    }

    fn import_curl(&mut self) {
        if let Ok(o) = Command::new("pbpaste").output() {
            let t = String::from_utf8_lossy(&o.stdout).to_string();
            if let Some(parsed) = crate::curl::parse_curl(&t) {
                let tab = self.current_tab_mut();
                tab.method = parsed.method;
                tab.url_area = TextArea::default(); tab.url_area.insert_str(&parsed.url);
                
                let mut h_str = String::new();
                for (k, v) in parsed.headers { h_str.push_str(&format!("{}: {}\n", k, v)); }
                tab.headers_area = TextArea::default(); tab.headers_area.insert_str(h_str.trim());
                
                tab.body_area = TextArea::default();
                if let Some(b) = parsed.body { 
                    tab.body_area.insert_str(&b); 
                    if b.starts_with('{') { tab.body_type = BodyType::Json; }
                }
                self.ai_response = "SYSTEM: cURL command imported successfully.".to_string();
            } else {
                self.ai_response = "SYSTEM ERROR: Clipboard does not contain a valid cURL command.".to_string();
            }
        }
    }

    fn cycle_editor_focus(&mut self) { let tab = self.current_tab_mut(); tab.editor_focus = match tab.editor_focus { EditorFocus::Url => EditorFocus::Headers, EditorFocus::Headers => EditorFocus::Body, EditorFocus::Body => EditorFocus::Attachment, EditorFocus::Attachment => EditorFocus::Url }; }
    fn cycle_method(&mut self, fwd: bool) { let ms = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]; let c = self.current_tab().method.clone(); let p = ms.iter().position(|&m| m == c).unwrap_or(0) as i32; let n = if fwd { (p + 1).rem_euclid(ms.len() as i32) } else { (p - 1).rem_euclid(ms.len() as i32) }; self.current_tab_mut().method = ms[n as usize].to_string(); }

        fn move_selection(&mut self, delta: i32) {

            match self.active_panel {

                ActivePanel::Collections => {

                    let count = match self.left_panel_tab {

                        LeftPanelTab::Collections => self.get_visible_items().len(),

                        LeftPanelTab::History => self.collections.history.len()

                    };

                    if count > 0 {

                        self.selected_idx = (self.selected_idx as i32 + delta).rem_euclid(count as i32) as usize;

                    }

                }

                ActivePanel::Response => {

     let t = self.current_tab_mut(); if delta > 0 { t.response_scroll = t.response_scroll.saturating_add(1); } else { t.response_scroll = t.response_scroll.saturating_sub(1); } }
            ActivePanel::Editor => {
                let tab = self.current_tab_mut();
                let key = if delta > 0 { KeyEvent::new(KeyCode::Down, KeyModifiers::empty()) } else { KeyEvent::new(KeyCode::Up, KeyModifiers::empty()) };
                match tab.editor_focus { EditorFocus::Url => { tab.url_area.input(key); }, EditorFocus::Headers => { tab.headers_area.input(key); }, EditorFocus::Body => { tab.body_area.input(key); }, _ => {} };
            }
            _ => {}
        }
    }

    fn load_selected_item(&mut self) {
        if matches!(self.left_panel_tab, LeftPanelTab::Collections) {
            let visible = self.get_visible_items();
            if let Some(item) = visible.get(self.selected_idx) {
                match item {
                    CollectionItem::Folder(name) => {
                        if self.expanded_groups.contains(name) {
                            self.expanded_groups.remove(name);
                        } else {
                            self.expanded_groups.insert(name.clone());
                        }
                    }
                    CollectionItem::Request(real_idx) => {
                        if let Some(req) = self.collections.requests.get(*real_idx).cloned() {
                            let t = self.current_tab_mut();
                            t.url_area = TextArea::default(); t.url_area.insert_str(&req.url);
                            t.headers_area = TextArea::default(); let h = req.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n"); t.headers_area.insert_str(h);
                            t.body_area = TextArea::default(); if let Some(b) = &req.body { t.body_area.insert_str(b); }
                            t.method = req.method.clone(); self.active_panel = ActivePanel::Editor;
                        }
                    }
                }
            }
        } else {
            // Historial (sigue siendo plano)
            if let Some(req) = self.collections.history.get(self.selected_idx).cloned() {
                let t = self.current_tab_mut();
                t.url_area = TextArea::default(); t.url_area.insert_str(&req.url);
                t.headers_area = TextArea::default(); let h = req.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n"); t.headers_area.insert_str(h);
                t.body_area = TextArea::default(); if let Some(b) = &req.body { t.body_area.insert_str(b); }
                t.method = req.method.clone(); self.active_panel = ActivePanel::Editor;
            }
        }
    }

        fn open_in_system(&mut self) {

            let tab = self.current_tab();

            if let Some(bytes) = &tab.response_bytes {

                let temp_path = std::env::temp_dir().join("arthema_resp.png");

                if std::fs::write(&temp_path, bytes).is_ok() {

                    let _ = Command::new("open").arg(&temp_path).spawn();

                    self.ai_response = "SYSTEM: Opening image in default viewer...".to_string();

                }

            } else if !tab.file_path.is_empty() {

                let _ = Command::new("open").arg(&tab.file_path).spawn();

            }

        }

    

        fn import_swagger(&mut self) {
        let url = self.swagger_input.lines()[0].trim().to_string();
        if url.is_empty() { return; }
        
        let tx = self.tx.clone();
        self.ai_response = "SYSTEM: Importing Swagger/OpenAPI spec...".to_string();
        
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(content) = resp.text().await {
                        let requests = crate::openapi::parse_swagger(&content);
                        let _ = tx.send(AppEvent::SwaggerImported(requests));
                    }
                }
                Err(e) => { let _ = tx.send(AppEvent::SystemMessage(format!("SWAGGER ERROR: {}", e))); }
            }
        });
        self.show_swagger_input = false;
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel { ActivePanel::Collections => ActivePanel::Editor, ActivePanel::Editor => ActivePanel::Response, ActivePanel::Response => ActivePanel::AI, _ => ActivePanel::Collections };
    }

    fn save_current_request(&mut self) {
        let t = self.current_tab();
        let mut hs = HashMap::new();
        for l in t.headers_area.lines() { let pts: Vec<&str> = l.splitn(2, ':').collect(); if pts.len() == 2 { hs.insert(pts[0].trim().to_string(), pts[1].trim().to_string()); } }
        let new_req = ApiRequest { 
            name: format!("Req_{}", self.collections.requests.len() + 1), 
            url: t.url_area.lines()[0].clone(), 
            method: t.method.clone(), 
            headers: hs, 
            body: Some(t.body_area.lines().join("\n")),
            group: None 
        };
        if self.collections.save_request(&new_req).is_ok() { let _ = self.collections.load_all(); self.ai_response = "SYSTEM: saved.".to_string(); }
    }

    pub fn send_request(&mut self) {
        let tx = self.tx.clone();
        let (url, m_str, body, h_lines, f_path) = {
            let t = self.current_tab_mut(); t.response = "SYNCING...".to_string(); t.response_bytes = None; t.response_scroll = 0;
            (t.url_area.lines()[0].clone(), t.method.clone(), t.body_area.lines().join("\n"), t.headers_area.lines().iter().map(|s| s.to_string()).collect::<Vec<_>>(), t.file_path.clone())
        };
        let mut h_map = HashMap::new();
        for l in h_lines { let p: Vec<&str> = l.splitn(2, ':').collect(); if p.len() == 2 { h_map.insert(p[0].trim().to_string(), p[1].trim().to_string()); } }
        self.collections.add_to_history(ApiRequest { 
            name: url.clone(), 
            url: url.clone(), 
            method: m_str.clone(), 
            headers: h_map.clone(), 
            body: Some(body.clone()),
            group: None 
        });

        tokio::spawn(async move {
            let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
            let method = match m_str.as_str() { "POST" => Method::POST, "PUT" => Method::PUT, "DELETE" => Method::DELETE, "PATCH" => Method::PATCH, "HEAD" => Method::HEAD, "OPTIONS" => Method::OPTIONS, _ => Method::GET };
            let mut rb = client.request(method.clone(), &url);
            for (k, v) in h_map { rb = rb.header(k, v); }
            if !f_path.is_empty() { if let Ok(b) = std::fs::read(&f_path) { let form = reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(b).file_name("upload")); rb = rb.multipart(form); } }
            else if !body.is_empty() && method != Method::GET { rb = rb.body(body); }
            
            match rb.header("User-Agent", "Arthema").send().await {
                Ok(resp) => {
                    let s = resp.status();
                    let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                    let bytes = resp.bytes().await.unwrap_or_default();
                    
                    if content_type.starts_with("image/") {
                        let _ = tx.send(AppEvent::ApiResponse(format!("STATUS: {}\nTYPE: {}\nSIZE: {} bytes", s, content_type, bytes.len()), Some(bytes.to_vec())));
                    } else {
                        let text = String::from_utf8_lossy(&bytes).to_string();
                        let fmtd = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) { serde_json::to_string_pretty(&val).unwrap_or(text) } else { text };
                        let _ = tx.send(AppEvent::ApiResponse(format!("STATUS: {}\n\n{}", s, fmtd), None));
                    }
                }
                Err(e) => { let _ = tx.send(AppEvent::SystemMessage(format!("ERROR: {}", e))); }
            }
        });
    }

    pub fn trigger_ai_suggestion(&mut self) { if !self.is_ai_loading { self.is_ai_loading = true; let tx = self.tx.clone(); let url = self.current_tab().url_area.lines()[0].clone(); let key = self.config.gemini_api_key.clone().unwrap_or_default(); tokio::spawn(async move { let s = crate::ai::get_ai_suggestion(&key, &url).await; let _ = tx.send(AppEvent::AiMessage(format!("AI_SUGGESTION:{}", s))); }); } }
    pub fn trigger_ai_explain(&mut self) { if !self.is_ai_loading { self.is_ai_loading = true; let tx = self.tx.clone(); let r = self.current_tab().response.clone(); let key = self.config.gemini_api_key.clone().unwrap_or_default(); tokio::spawn(async move { let e = crate::ai::explain_response(&key, &r).await; let _ = tx.send(AppEvent::AiMessage(format!("AI_EXPLANATION:{}", e))); }); } }
    pub fn trigger_ai_fix(&mut self) { if !self.is_ai_loading { self.is_ai_loading = true; let t = self.current_tab(); let tx = self.tx.clone(); let (m, u, h, b, e) = (t.method.clone(), t.url_area.lines()[0].clone(), t.headers_area.lines().join("\n"), t.body_area.lines().join("\n"), t.response.clone()); let key = self.config.gemini_api_key.clone().unwrap_or_default(); tokio::spawn(async move { let e = crate::ai::fix_error(&key, &m, &u, &h, &b, &e).await; let _ = tx.send(AppEvent::AiMessage(format!("AI_EXPLANATION:{}", e))); }); } }

    pub fn update(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.is_ai_loading = false;
            match event {
                AppEvent::ApiResponse(text, bytes) => {
                    let t = self.current_tab_mut();
                    t.response = text;
                    t.response_bytes = bytes;
                }
                AppEvent::AiMessage(res) => {
                    if let Some(s) = res.strip_prefix("AI_SUGGESTION:") {
                        let t = self.current_tab_mut();
                        t.url_area = TextArea::default();
                        t.url_area.insert_str(s.trim());
                    } else if let Some(e) = res.strip_prefix("AI_EXPLANATION:") {
                        self.ai_response = e.to_string();
                    }
                }
                AppEvent::SwaggerImported(reqs) => {
                    let count = reqs.len();
                    for r in reqs {
                        let _ = self.collections.save_request(&r);
                    }
                    let _ = self.collections.load_all();
                    self.ai_response = format!("SYSTEM: Imported {} requests from Swagger.", count);
                }
                AppEvent::SystemMessage(msg) => {
                    self.current_tab_mut().response = msg;
                }
            }
        }
        if self.last_sys_update.elapsed() > Duration::from_secs(2) {
            self.sys.refresh_cpu(); self.sys.refresh_memory();
            self.cpu_usage = self.sys.global_cpu_info().cpu_usage(); self.mem_total = self.sys.total_memory() / 1024 / 1024; self.mem_used = self.sys.used_memory() / 1024 / 1024;
            let pid = Pid::from_u32(std::process::id()); self.sys.refresh_process(pid);
            if let Some(proc) = self.sys.process(pid) { let num_cpus = self.sys.cpus().len() as f32; self.proc_cpu = proc.cpu_usage() / num_cpus; self.proc_mem = proc.memory() / 1024 / 1024; }
            if let Ok(output) = Command::new("pmset").arg("-g").arg("batt").output() { let out = String::from_utf8_lossy(&output.stdout); if let Some(line) = out.lines().nth(1) { if let Some(perc) = line.split('\t').nth(1) { self.battery_level = perc.split(';').next().unwrap_or("N/A").to_string(); } } }
            self.last_sys_update = Instant::now();
        }
    }
}

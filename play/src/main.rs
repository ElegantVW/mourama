//! Native Mourama window. The server is the world; this is a seat.

use eframe::egui::{self, Color32, ColorImage, Pos2, Sense, Stroke, TextureHandle, Vec2};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PINK: Color32 = Color32::from_rgb(0xe8, 0x79, 0xa0);
const PINK_D: Color32 = Color32::from_rgb(0xc4, 0x4d, 0x7a);
const BLUSH: Color32 = Color32::from_rgb(0xf0, 0xb4, 0xc8);
const BG: Color32 = Color32::from_rgb(0x1a, 0x0a, 0x12);
const PANEL: Color32 = Color32::from_rgb(0x2a, 0x12, 0x20);
const SILVER: Color32 = Color32::from_rgb(0xc0, 0xc0, 0xc8);
const GOLD: Color32 = Color32::from_rgb(0xff, 0xb0, 0x20);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x2d, 0x55);

const UNITS: &[&str] = &["pastor", "trasgo", "falcao", "javali", "guerreiro", "serpe"];
const TUTORIAL: &[&str] = &[
    "You claimed a citânia. This hill is yours.",
    "Four piles: cobre, estanho, seara, orvalho. Cobre+estanho spend as bronze.",
    "Raise the Eira to grow seara. Watch the timer on the row.",
    "Train a Pastor. Steppers start at 0 — you choose how many walk.",
    "Open the Wood. Pink hex is you. Silver is a mamoa. Dim is empty.",
    "Encanto veils your numbers. A falcão scout sees the truth.",
    "A serpe plus orvalho binds a hill. Do not dump the whole garrison.",
    "Five seats. The host mints a one-time invite with mourama invite.",
];

#[derive(Clone, Serialize, Deserialize)]
struct Config {
    server: String,
    token: String,
    tutorial_step: i32,
    #[serde(default)]
    tutorial_done: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: "http://192.168.8.186:4747".into(),
            token: String::new(),
            tutorial_step: 0,
            tutorial_done: false,
        }
    }
}

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/mourama/play.json")
}

fn load_config() -> Config {
    let p = config_path();
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(c: &Config) {
    if let Some(dir) = config_path().parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(s) = serde_json::to_string_pretty(c) {
        let _ = std::fs::write(config_path(), s);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(config_path(), std::fs::Permissions::from_mode(0o600));
        }
    }
}

fn icons_dir() -> PathBuf {
    if let Ok(p) = std::env::var("MOURAMA_ICONS") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let c = dir.join("icons");
            if c.join("app.png").is_file() {
                return c;
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    for c in [
        PathBuf::from(&home).join(".local/lib/faeos/mourama-icons"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/icons"),
        PathBuf::from(&home).join("mourama/assets/icons"),
    ] {
        if c.join("app.png").is_file() {
            return c;
        }
    }
    PathBuf::from("assets/icons")
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn eta(ts: i64) -> String {
    let d = ts - now_unix();
    if d <= 0 {
        return "now".into();
    }
    let m = d / 60;
    let s = d % 60;
    if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

fn fnum(v: &Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_i64().map(|n| n as f64))
        .unwrap_or(0.0)
}

struct PlayApp {
    cfg: Config,
    icons: HashMap<String, TextureHandle>,
    tab: Tab,
    gate_invite: String,
    gate_name: String,
    gate_pass: String,
    gate_err: String,
    me: Option<Value>,
    map: Option<Value>,
    reports: Option<Value>,
    last_pull: Instant,
    pull_err: String,
    hill_idx: usize,
    send_q: i32,
    send_r: i32,
    send_counts: HashMap<String, i32>,
    train_counts: HashMap<String, i32>,
    mission: String,
    confirm: Option<String>,
    map_pan: Vec2,
    map_zoom: f32,
    dragging: bool,
    last_pointer: Option<Pos2>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Hill,
    Wood,
    Reports,
}

impl PlayApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = PANEL;
        visuals.override_text_color = Some(Color32::from_rgb(0xff, 0xeb, 0xf2));
        visuals.selection.bg_fill = PINK_D;
        visuals.widgets.inactive.bg_fill = PANEL;
        visuals.widgets.hovered.bg_fill = PINK_D;
        visuals.widgets.active.bg_fill = PINK;
        cc.egui_ctx.set_visuals(visuals);

        let mut icons = HashMap::new();
        let dir = icons_dir();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.extension().and_then(|e| e.to_str()) != Some("png") {
                    continue;
                }
                let stem = p.file_stem().unwrap().to_string_lossy().to_string();
                if let Ok(img) = image::open(&p) {
                    let rgba = img.into_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let tex = cc.egui_ctx.load_texture(
                        &stem,
                        ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
                        egui::TextureOptions::LINEAR,
                    );
                    icons.insert(stem, tex);
                }
            }
        }

        let mut app = Self {
            cfg: load_config(),
            icons,
            tab: Tab::Hill,
            gate_invite: String::new(),
            gate_name: String::new(),
            gate_pass: String::new(),
            gate_err: String::new(),
            me: None,
            map: None,
            reports: None,
            last_pull: Instant::now() - Duration::from_secs(10),
            pull_err: String::new(),
            hill_idx: 0,
            send_q: 0,
            send_r: 0,
            send_counts: UNITS.iter().map(|u| (u.to_string(), 0)).collect(),
            train_counts: UNITS.iter().map(|u| (u.to_string(), 0)).collect(),
            mission: "raid".into(),
            confirm: None,
            map_pan: Vec2::ZERO,
            map_zoom: 1.0,
            dragging: false,
            last_pointer: None,
        };
        if !app.cfg.token.is_empty() {
            app.refresh();
        }
        app
    }

    fn icon(&self, key: &str, ui: &mut egui::Ui, size: f32) {
        if let Some(tex) = self.icons.get(key) {
            ui.add(egui::Image::new(tex).max_size(Vec2::splat(size)));
        }
    }

    fn api(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
        let url = format!("{}{path}", self.cfg.server.trim_end_matches('/'));
        let mut req = match method {
            "POST" => ureq::post(&url),
            _ => ureq::get(&url),
        };
        req = req.timeout(Duration::from_secs(8));
        if !self.cfg.token.is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", self.cfg.token));
        }
        let resp = if let Some(b) = body {
            req.set("content-type", "application/json")
                .send_json(b)
        } else {
            req.call()
        }
        .map_err(|e| format!("mourama: {e}"))?;
        resp.into_json::<Value>()
            .map_err(|e| format!("mourama: {e}"))
    }

    fn err_of(v: &Value) -> Option<String> {
        v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string())
    }

    fn refresh(&mut self) {
        match self.api("GET", "/api/me", None) {
            Ok(v) => {
                if let Some(e) = Self::err_of(&v) {
                    self.pull_err = e;
                    self.cfg.token.clear();
                } else {
                    self.me = Some(v);
                    self.pull_err.clear();
                }
            }
            Err(e) => self.pull_err = e,
        }
        if let Ok(v) = self.api("GET", "/api/map", None) {
            self.map = Some(v);
        }
        if let Ok(v) = self.api("GET", "/api/reports", None) {
            self.reports = Some(v);
        }
        self.last_pull = Instant::now();
    }

    fn current_hill(&self) -> Option<&Value> {
        let hills = self.me.as_ref()?.get("hills")?.as_array()?;
        hills.get(self.hill_idx).or_else(|| hills.first())
    }

    fn post_ok(&mut self, path: &str, body: Value) {
        match self.api("POST", path, Some(body)) {
            Ok(v) => {
                if let Some(e) = Self::err_of(&v) {
                    self.pull_err = e;
                } else {
                    self.pull_err.clear();
                    self.refresh();
                }
            }
            Err(e) => self.pull_err = e,
        }
    }
}

impl eframe::App for PlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));
        if !self.cfg.token.is_empty() && self.last_pull.elapsed() > Duration::from_secs(2) {
            self.refresh();
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                self.icon("app", ui, 28.0);
                ui.label(egui::RichText::new("mourama").color(PINK).size(20.0).strong());
                ui.separator();
                if let Some(me) = &self.me {
                    ui.label(egui::RichText::new(me["name"].as_str().unwrap_or("")).color(BLUSH));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("?").clicked() {
                        self.cfg.tutorial_done = false;
                        self.cfg.tutorial_step = 0;
                        save_config(&self.cfg);
                    }
                    ui.label(egui::RichText::new(&self.cfg.server).color(SILVER).small());
                });
            });
            if !self.pull_err.is_empty() {
                ui.colored_label(DANGER, &self.pull_err);
            }
            ui.add_space(4.0);
        });

        if self.cfg.token.is_empty() {
            self.ui_gate(ctx);
            return;
        }

        if let Some(hill) = self.current_hill().cloned() {
            egui::TopBottomPanel::top("res").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (key, label, field) in [
                        ("res-cobre", "Cobre", "cobre"),
                        ("res-estanho", "Estanho", "estanho"),
                        ("res-seara", "Seara", "seara"),
                        ("res-orvalho", "Orvalho", "orvalho"),
                    ] {
                        self.icon(key, ui, 28.0);
                        let n = fnum(&hill["resources"][field]);
                        ui.label(format!("{label}  {}", n.floor()));
                        ui.add_space(12.0);
                    }
                    self.icon("res-bronze", ui, 28.0);
                    ui.colored_label(GOLD, format!("Bronze  {}", fnum(&hill["bronze"]).floor()));
                });
            });
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Hill, "Citânia");
                ui.selectable_value(&mut self.tab, Tab::Wood, "Wood");
                ui.selectable_value(&mut self.tab, Tab::Reports, "Reports");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Hill => self.ui_hill(ui),
            Tab::Wood => self.ui_wood(ui),
            Tab::Reports => self.ui_reports(ui),
        });

        if !self.cfg.tutorial_done {
            self.ui_tutorial(ctx);
        }

        if let Some(msg) = self.confirm.clone() {
            egui::Window::new("Confirm")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(&msg);
                    ui.horizontal(|ui| {
                        if ui.button("Send them").clicked() {
                            self.do_send();
                            self.confirm = None;
                        }
                        if ui.button("Stay").clicked() {
                            self.confirm = None;
                        }
                    });
                });
        }
    }
}

impl PlayApp {
    fn ui_gate(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(24.0);
            ui.label(egui::RichText::new("Five seats. Iberian hills. The office box is the world.").color(BLUSH));
            ui.add_space(12.0);
            ui.label("Server");
            ui.text_edit_singleline(&mut self.cfg.server);
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Claim a seat").color(PINK));
                ui.label("Invite");
                ui.text_edit_singleline(&mut self.gate_invite);
                ui.label("Court name");
                ui.text_edit_singleline(&mut self.gate_name);
                ui.label("Word");
                ui.add(egui::TextEdit::singleline(&mut self.gate_pass).password(true));
                if ui.button("Claim").clicked() {
                    save_config(&self.cfg);
                    match self.api(
                        "POST",
                        "/api/claim",
                        Some(json!({
                            "invite": self.gate_invite,
                            "name": self.gate_name,
                            "password": self.gate_pass,
                        })),
                    ) {
                        Ok(v) => {
                            if let Some(e) = Self::err_of(&v) {
                                self.gate_err = e;
                            } else if let Some(t) = v["token"].as_str() {
                                self.cfg.token = t.to_string();
                                self.cfg.tutorial_done = false;
                                self.cfg.tutorial_step = 0;
                                save_config(&self.cfg);
                                self.refresh();
                            }
                        }
                        Err(e) => self.gate_err = e,
                    }
                }
            });
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Already seated").color(PINK));
                ui.label("Court name");
                ui.text_edit_singleline(&mut self.gate_name);
                ui.label("Word");
                ui.add(egui::TextEdit::singleline(&mut self.gate_pass).password(true));
                if ui.button("Sit down").clicked() {
                    save_config(&self.cfg);
                    match self.api(
                        "POST",
                        "/api/login",
                        Some(json!({
                            "name": self.gate_name,
                            "password": self.gate_pass,
                        })),
                    ) {
                        Ok(v) => {
                            if let Some(e) = Self::err_of(&v) {
                                self.gate_err = e;
                            } else if let Some(t) = v["token"].as_str() {
                                self.cfg.token = t.to_string();
                                save_config(&self.cfg);
                                self.refresh();
                            }
                        }
                        Err(e) => self.gate_err = e,
                    }
                }
            });
            if !self.gate_err.is_empty() {
                ui.colored_label(DANGER, &self.gate_err);
            }
        });
    }

    fn ui_hill(&mut self, ui: &mut egui::Ui) {
        let Some(me) = self.me.clone() else {
            ui.label("sitting down…");
            return;
        };
        let hills = me["hills"].as_array().cloned().unwrap_or_default();
        if hills.is_empty() {
            ui.label("Your court holds no hill.");
            return;
        }
        ui.horizontal(|ui| {
            for (i, h) in hills.iter().enumerate() {
                let name = h["name"].as_str().unwrap_or("hill");
                if ui.selectable_label(self.hill_idx == i, name).clicked() {
                    self.hill_idx = i;
                }
            }
        });
        let hill = hills.get(self.hill_idx).unwrap_or(&hills[0]).clone();
        let cid = hill["id"].as_i64().unwrap_or(0);
        ui.label(format!(
            "{}  ({},{})",
            hill["name"].as_str().unwrap_or(""),
            hill["q"],
            hill["r"]
        ));
        let enc = hill["encanto_until"].as_i64().unwrap_or(0);
        if enc > now_unix() {
            ui.colored_label(GOLD, format!("encanto until {}", eta(enc)));
        } else if ui.button("Cast encanto").clicked() {
            self.post_ok("/api/encanto", json!({ "castro_id": cid }));
        }

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Buildings").color(PINK));
        if let Some(obj) = hill["buildings"].as_object() {
            let mut keys: Vec<_> = obj.keys().cloned().collect();
            keys.sort();
            for k in keys {
                let b = &obj[&k];
                ui.horizontal(|ui| {
                    self.icon(&format!("bldg-{k}"), ui, 28.0);
                    let lvl = b["level"].as_i64().unwrap_or(0);
                    ui.label(format!("{}  L{lvl}", b["title"].as_str().unwrap_or(&k)));
                    if let Some(done) = b["upgrade_done"].as_i64() {
                        ui.colored_label(GOLD, format!("rising {}", eta(done)));
                    } else {
                        let c = &b["next_cost"];
                        ui.label(
                            egui::RichText::new(format!(
                                "cobre {:.0} · estanho {:.0} · seara {:.0} · orvalho {:.0} · bronze {:.0} · {}",
                                fnum(&c["cobre"]),
                                fnum(&c["estanho"]),
                                fnum(&c["seara"]),
                                fnum(&c["orvalho"]),
                                fnum(&b["next_bronze"]),
                                eta(now_unix() + b["next_secs"].as_i64().unwrap_or(0)),
                            ))
                            .small()
                            .color(SILVER),
                        );
                        if ui.button("Raise").clicked() {
                            self.post_ok(
                                "/api/upgrade",
                                json!({ "castro_id": cid, "building": k }),
                            );
                        }
                    }
                });
            }
        }

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Garrison").color(PINK));
        let g = hill["garrison"].as_object().cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            for u in UNITS {
                self.icon(&format!("unit-{u}"), ui, 24.0);
                let n = g.get(*u).and_then(|v| v.as_i64()).unwrap_or(0);
                ui.label(format!("{u} {n}"));
                ui.add_space(8.0);
            }
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Train").color(PINK));
        let catalog_units = me
            .pointer("/catalog/units")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for spec in catalog_units {
            let id = spec["id"].as_str().unwrap_or("").to_string();
            let title = spec["title"].as_str().unwrap_or(&id).to_string();
            let count = {
                let n = self.train_counts.entry(id.clone()).or_insert(0);
                ui.horizontal(|ui| {
                    ui.label(&title);
                    ui.add(egui::Slider::new(n, 0..=50).text(""));
                });
                *n
            };
            ui.horizontal(|ui| {
                self.icon(&format!("unit-{id}"), ui, 28.0);
                let c = &spec["cost"];
                ui.label(
                    egui::RichText::new(format!(
                        "×{count}  seara {:.0} cobre {:.0} orvalho {:.0} bronze {:.0}  {}",
                        fnum(&c["seara"]) * count as f64,
                        fnum(&c["cobre"]) * count as f64,
                        fnum(&c["orvalho"]) * count as f64,
                        fnum(&spec["bronze"]) * count as f64,
                        eta(now_unix() + spec["secs"].as_i64().unwrap_or(0) * count as i64),
                    ))
                    .small()
                    .color(SILVER),
                );
                if ui.button("Train").clicked() && count > 0 {
                    self.post_ok(
                        "/api/train",
                        json!({ "castro_id": cid, "unit": id, "count": count }),
                    );
                }
            });
        }

        if let Some(kind) = hill["train_kind"].as_str() {
            if let Some(done) = hill["train_done"].as_i64() {
                ui.colored_label(
                    GOLD,
                    format!(
                        "curral raising {} ×{} — {}",
                        kind,
                        hill["train_count"],
                        eta(done)
                    ),
                );
            }
        }

        ui.add_space(8.0);
        ui.label(egui::RichText::new("On the between").color(PINK));
        if let Some(armies) = me["armies"].as_array() {
            for a in armies {
                let mission = a["mission"].as_str().unwrap_or("?");
                let arrive = a["arrive"].as_i64().unwrap_or(0);
                ui.label(format!(
                    "{mission} → ({},{})  {}",
                    a["to"][0],
                    a["to"][1],
                    eta(arrive)
                ));
            }
        }
    }

    fn ui_wood(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.icon("tile-citania", ui, 18.0);
            ui.label("you");
            self.icon("tile-mamoa", ui, 18.0);
            ui.label("mamoa");
            self.icon("tile-outeiro", ui, 18.0);
            ui.label("empty");
            self.icon("tile-veiled", ui, 18.0);
            ui.label("encanto");
        });
        let Some(map) = self.map.clone() else {
            ui.label("the mist has not lifted");
            return;
        };
        let tiles = map["tiles"].as_array().cloned().unwrap_or_default();
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), (ui.available_height() - 160.0).max(280.0)),
            Sense::click_and_drag(),
        );
        let rect = response.rect;
        if response.dragged() {
            if let Some(prev) = self.last_pointer {
                if let Some(now) = response.interact_pointer_pos() {
                    self.map_pan += now - prev;
                }
            }
            self.dragging = true;
        } else {
            self.dragging = false;
        }
        self.last_pointer = response.interact_pointer_pos();
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if response.hovered() && scroll.abs() > 0.0 {
            self.map_zoom = (self.map_zoom * (1.0 + scroll * 0.002)).clamp(0.6, 2.4);
        }

        let size = 16.0 * self.map_zoom;
        let origin = rect.center() + self.map_pan;
        let mut hit: Option<(i32, i32, Value)> = None;
        for t in &tiles {
            let q = t["q"].as_i64().unwrap_or(0) as i32;
            let r = t["r"].as_i64().unwrap_or(0) as i32;
            let x = size * 3.0_f32.sqrt() * (q as f32 + r as f32 / 2.0);
            let y = size * 1.5 * r as f32;
            let c = origin + Vec2::new(x, y);
            let fill = if t["mine"].as_bool().unwrap_or(false) {
                PINK
            } else if t["veiled"].as_bool().unwrap_or(false) {
                GOLD
            } else {
                match t["kind"].as_str().unwrap_or("") {
                    "mamoa" => SILVER,
                    "castro" | "seat" => PINK_D,
                    _ => Color32::from_rgb(0x3a, 0x24, 0x30),
                }
            };
            let pts = hex_pts(c, size);
            painter.add(egui::Shape::convex_polygon(pts, fill, Stroke::new(1.0_f32, BG)));
            if let Some(pos) = response.interact_pointer_pos() {
                if c.distance(pos) < size * 0.9 {
                    hit = Some((q, r, t.clone()));
                }
            }
        }
        if response.clicked() {
            if let Some((q, r, _)) = &hit {
                self.send_q = *q;
                self.send_r = *r;
            }
        }

        ui.separator();
        ui.label(format!("selected ({}, {})", self.send_q, self.send_r));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mission, "raid".into(), "raid");
            ui.selectable_value(&mut self.mission, "scout".into(), "scout");
            ui.selectable_value(&mut self.mission, "bind".into(), "bind");
        });
        if self.mission == "scout" {
            ui.colored_label(SILVER, "Scout is falcão only.");
        }
        if self.mission == "bind" {
            ui.colored_label(SILVER, "Bind needs a serpe and 100 orvalho.");
        }
        let g = self
            .current_hill()
            .and_then(|h| h["garrison"].as_object().cloned())
            .unwrap_or_default();
        for u in UNITS {
            let have = g.get(*u).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            ui.horizontal(|ui| {
                self.icon(&format!("unit-{u}"), ui, 24.0);
                ui.label(format!("{u}  hold {have}"));
                let n = self.send_counts.entry((*u).into()).or_insert(0);
                ui.add(egui::Slider::new(n, 0..=have.max(0)).text("send"));
            });
        }
        if ui.button("Send across the between").clicked() {
            let total: i32 = self.send_counts.values().sum();
            if total <= 0 {
                self.pull_err = "mourama: send someone, not an empty mist.".into();
            } else {
                self.confirm = Some(format!(
                    "Send {total} folk on a {} to ({}, {})?",
                    self.mission, self.send_q, self.send_r
                ));
            }
        }
        let _ = rect;
    }

    fn do_send(&mut self) {
        let Some(hill) = self.current_hill().cloned() else {
            return;
        };
        let cid = hill["id"].as_i64().unwrap_or(0);
        let mut units = serde_json::Map::new();
        for (k, n) in &self.send_counts {
            if *n > 0 {
                units.insert(k.clone(), json!(*n));
            }
        }
        self.post_ok(
            "/api/send",
            json!({
                "castro_id": cid,
                "q": self.send_q,
                "r": self.send_r,
                "units": units,
                "mission": self.mission,
            }),
        );
        for n in self.send_counts.values_mut() {
            *n = 0;
        }
    }

    fn ui_reports(&mut self, ui: &mut egui::Ui) {
        let Some(rep) = self.reports.clone() else {
            ui.label("no reports");
            return;
        };
        let rows = rep["reports"].as_array().cloned().unwrap_or_default();
        if rows.is_empty() {
            ui.label("No reports yet.");
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for r in rows {
                ui.group(|ui| {
                    ui.colored_label(PINK, r["title"].as_str().unwrap_or(""));
                    let text = r["body"]["text"]
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| readable_report(&r));
                    ui.label(text);
                });
            }
        });
    }

    fn ui_tutorial(&mut self, ctx: &egui::Context) {
        let step = self.cfg.tutorial_step.clamp(0, TUTORIAL.len() as i32 - 1) as usize;
        egui::Window::new("How the wood works")
            .anchor(egui::Align2::LEFT_BOTTOM, [16.0, -16.0])
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(format!("{}/{}", step + 1, TUTORIAL.len())).color(SILVER).small());
                ui.label(TUTORIAL[step]);
                ui.horizontal(|ui| {
                    if ui.button("Next").clicked() {
                        if step + 1 >= TUTORIAL.len() {
                            self.cfg.tutorial_done = true;
                        } else {
                            self.cfg.tutorial_step = (step + 1) as i32;
                        }
                        save_config(&self.cfg);
                    }
                    if ui.button("Skip").clicked() {
                        self.cfg.tutorial_done = true;
                        save_config(&self.cfg);
                    }
                });
            });
    }
}

fn readable_report(r: &Value) -> String {
    let title = r["title"].as_str().unwrap_or("report");
    let body = &r["body"];
    if let Some(t) = body["text"].as_str() {
        return t.to_string();
    }
    if body["won"].as_bool() == Some(true) {
        return format!("{title} — the hill yielded.");
    }
    if body["won"].as_bool() == Some(false) {
        return format!("{title} — they held.");
    }
    if body.get("seen").is_some() {
        return format!("{title} — the falcão looked through.");
    }
    title.to_string()
}

fn hex_pts(c: Pos2, size: f32) -> Vec<Pos2> {
    (0..6)
        .map(|i| {
            let a = std::f32::consts::PI / 6.0 + i as f32 * std::f32::consts::PI / 3.0;
            Pos2::new(c.x + size * a.cos(), c.y + size * a.sin())
        })
        .collect()
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([860.0, 560.0])
            .with_title("Mourama"),
        ..Default::default()
    };
    eframe::run_native(
        "Mourama",
        opts,
        Box::new(|cc| Ok(Box::new(PlayApp::new(cc)))),
    )
}

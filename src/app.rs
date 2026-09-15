use std::time::Duration;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use walkupie::settings::{self, Settings};
use walkupie::sound::Sound;
use walkupie::timer::{fmt_mmss, Phase, Timer, Transition};

const WIDGET_W: f32 = 176.0;
const WIDGET_H: f32 = 200.0;

const WORK_COLOR: egui::Color32 = egui::Color32::from_rgb(46, 194, 110);
const BREAK_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 184, 64);
const PANEL_FILL: egui::Color32 = egui::Color32::from_rgb(26, 28, 34);
const TEXT_COLOR: egui::Color32 = egui::Color32::from_gray(225);

#[derive(Clone, Copy, PartialEq)]
enum BtnIcon {
    Play,
    Pause,
    Skip,
    Gear,
    HideTray,
}

fn paint_icon(painter: &egui::Painter, icon: BtnIcon, rect: egui::Rect, color: egui::Color32) {
    let c = rect.center();
    let s = rect.width();
    match icon {
        BtnIcon::Play => {
            let pts = vec![
                egui::pos2(c.x - s * 0.24, c.y - s * 0.32),
                egui::pos2(c.x - s * 0.24, c.y + s * 0.32),
                egui::pos2(c.x + s * 0.30, c.y),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
        }
        BtnIcon::Pause => {
            let bw = s * 0.12;
            let gap = s * 0.10;
            let y0 = c.y - s * 0.32;
            let y1 = c.y + s * 0.32;
            painter.rect_filled(
                egui::Rect::from_two_pos(
                    egui::pos2(c.x - gap - bw, y0),
                    egui::pos2(c.x - gap, y1),
                ),
                egui::CornerRadius::same(1),
                color,
            );
            painter.rect_filled(
                egui::Rect::from_two_pos(
                    egui::pos2(c.x + gap, y0),
                    egui::pos2(c.x + gap + bw, y1),
                ),
                egui::CornerRadius::same(1),
                color,
            );
        }
        BtnIcon::Skip => {
            let h = s * 0.32;
            let w = s * 0.26;
            for off in [-s * 0.14, s * 0.16] {
                let x0 = c.x + off;
                let pts = vec![
                    egui::pos2(x0, c.y - h),
                    egui::pos2(x0, c.y + h),
                    egui::pos2(x0 + w, c.y),
                ];
                painter.add(egui::Shape::convex_polygon(pts, color, egui::Stroke::NONE));
            }
        }
        BtnIcon::Gear => {
            let r = s * 0.42;
            painter.circle_stroke(c, r, egui::Stroke::new(1.8, color));
            painter.circle_filled(c, s * 0.09, color);
            for k in 0..8 {
                let a = std::f32::consts::TAU * (k as f32 / 8.0);
                let (sin, cos) = a.sin_cos();
                let p1 = egui::pos2(c.x + r * cos, c.y + r * sin);
                let p2 = egui::pos2(c.x + (r + s * 0.14) * cos, c.y + (r + s * 0.14) * sin);
                painter.line_segment([p1, p2], egui::Stroke::new(1.8, color));
            }
        }
        BtnIcon::HideTray => {
            let y = c.y + s * 0.34;
            painter.line_segment(
                [egui::pos2(c.x - s * 0.30, y), egui::pos2(c.x + s * 0.30, y)],
                egui::Stroke::new(2.0, color),
            );
            let tip = egui::pos2(c.x, c.y + s * 0.14);
            painter.line_segment(
                [egui::pos2(c.x, c.y - s * 0.26), tip],
                egui::Stroke::new(2.0, color),
            );
            painter.line_segment(
                [tip, egui::pos2(c.x - s * 0.16, c.y + s * 0.06)],
                egui::Stroke::new(2.0, color),
            );
            painter.line_segment(
                [tip, egui::pos2(c.x + s * 0.16, c.y + s * 0.06)],
                egui::Stroke::new(2.0, color),
            );
        }
    }
}

fn icon_button(ui: &mut egui::Ui, icon: BtnIcon, tooltip: &str, size: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    let painter = ui.painter();
    let visuals = ui.visuals();
    if resp.hovered() {
        painter.rect_filled(
            rect.expand(3.0),
            egui::CornerRadius::same(6),
            visuals.widgets.hovered.weak_bg_fill,
        );
    }
    let color = if resp.hovered() {
        visuals.strong_text_color()
    } else {
        visuals.text_color()
    };
    paint_icon(&painter, icon, rect, color);
    resp.on_hover_text(tooltip).clicked()
}

struct TrayItems {
    show: MenuItem,
    pause: MenuItem,
    settings: MenuItem,
    quit: MenuItem,
}

pub struct WalkUpie {
    timer: Timer,
    settings: Settings,
    draft: Settings,
    settings_open: bool,
    overlay_open: bool,
    minimized: bool,
    prev_minimized: bool,
    placed: bool,
    quitting: bool,
    tray: Option<TrayIcon>,
    tray_items: Option<TrayItems>,
    sound: Option<Sound>,
    last_tooltip: String,
}

impl WalkUpie {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let work = Duration::from_secs(settings.work_minutes.saturating_mul(60));
        let brk = Duration::from_secs(settings.break_minutes.saturating_mul(60));

        let timer = match (
            settings.timer_phase.as_deref().and_then(Phase::from_str),
            settings.timer_remaining_secs,
        ) {
            (Some(phase), Some(secs)) if secs > 0 => {
                Timer::restore(phase, Duration::from_secs(secs), work, brk)
            }
            _ => Timer::new(work, brk),
        };

        let mut app = Self {
            timer,
            settings,
            draft: Settings::default(),
            settings_open: false,
            overlay_open: false,
            minimized: false,
            prev_minimized: false,
            placed: false,
            quitting: false,
            tray: None,
            tray_items: None,
            sound: Sound::new(),
            last_tooltip: String::new(),
        };
        app.draft = app.settings.clone();
        app.setup_tray();
        app
    }

    fn setup_tray(&mut self) {
        let icon = Self::make_icon();

        let menu = Menu::new();
        let show = MenuItem::new("Hide Widget", true, None);
        let pause = MenuItem::new("Pause", true, None);
        let settings_item = MenuItem::new("Settings…", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let sep = PredefinedMenuItem::separator();

        for item in [&show, &pause, &settings_item, &quit] {
            if let Err(e) = menu.append(item) {
                log::warn!("tray append: {e}");
                return;
            }
        }
        if let Err(e) = menu.append(&sep) {
            log::warn!("tray append: {e}");
        }

        let tray = match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("WalkUpie")
            .build()
        {
            Ok(t) => t,
            Err(e) => {
                log::warn!("tray icon: {e}");
                return;
            }
        };

        self.tray = Some(tray);
        self.tray_items = Some(TrayItems {
            show,
            pause,
            settings: settings_item,
            quit,
        });
    }

    fn make_icon() -> Icon {
        let w = 32usize;
        let h = 32usize;
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let dx = (x as f32 - 15.5) / 16.0;
                let dy = (y as f32 - 15.5) / 16.0;
                let d = (dx * dx + dy * dy).sqrt();
                if d <= 1.0 {
                    let idx = (y * w + x) * 4;
                    rgba[idx + 3] = 255;
                    rgba[idx] = 34;
                    rgba[idx + 1] = 194;
                    rgba[idx + 2] = 110;
                    if d > 0.52 && d < 0.82 || d < 0.22 {
                        rgba[idx] = 255;
                        rgba[idx + 1] = 255;
                        rgba[idx + 2] = 255;
                    }
                }
            }
        }
        Icon::from_rgba(rgba, w as u32, h as u32)
            .unwrap_or_else(|_| Icon::from_rgba(vec![0; 16 * 16 * 4], 16, 16).unwrap())
    }

    fn persist(&mut self) {
        self.settings.timer_phase = Some(self.timer.phase.to_str().to_string());
        self.settings.timer_remaining_secs = Some(self.timer.remaining().as_secs());
        if let Err(e) = settings::save(&self.settings) {
            log::warn!("settings save failed: {e}");
        }
    }

    fn handle_tray(&mut self, ctx: &egui::Context) {
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                self.toggle_widget();
            }
        }
        loop {
            match MenuEvent::receiver().try_recv() {
                Ok(event) => {
                    let Some(items) = &self.tray_items else {
                        break;
                    };
                    if event.id == items.show.id() {
                        self.toggle_widget();
                    } else if event.id == items.pause.id() {
                        self.timer.toggle_pause();
                    } else if event.id == items.settings.id() {
                        self.settings_open = true;
                        self.draft = self.settings.clone();
                    } else if event.id == items.quit.id() {
                        self.quit(ctx);
                    }
                }
                Err(_) => break,
            }
        }
        if let Some(items) = &self.tray_items {
            let pause_label = if self.timer.is_paused() {
                "Resume"
            } else {
                "Pause"
            };
            items.pause.set_text(pause_label);
            let show_label = if self.minimized {
                "Show Widget"
            } else {
                "Hide Widget"
            };
            items.show.set_text(show_label);
        }
        let tooltip = format!(
            "WalkUpie — {} · {}",
            self.timer.phase.label(),
            fmt_mmss(self.timer.remaining())
        );
        if tooltip != self.last_tooltip {
            self.last_tooltip = tooltip.clone();
            if let Some(tray) = &self.tray {
                let _ = tray.set_tooltip(Some(tooltip));
            }
        }
    }

    fn toggle_widget(&mut self) {
        self.minimized = !self.minimized;
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        self.persist();
        ctx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close);
    }

    fn save_settings(&mut self, ctx: &egui::Context) {
        self.draft.work_minutes = self.draft.work_minutes.clamp(1, 24 * 60);
        self.draft.break_minutes = self.draft.break_minutes.clamp(1, 120);
        self.settings = self.draft.clone();
        self.settings_open = false;
        self.timer.set_durations(
            Duration::from_secs(self.settings.work_minutes * 60),
            Duration::from_secs(self.settings.break_minutes * 60),
        );
        let level = if self.settings.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::WindowLevel(level),
        );
        if let Ok(exe) = std::env::current_exe() {
            let _ = settings::set_auto_start(self.settings.auto_start, &exe);
        }
        self.persist();
    }

    fn chime(&self) {
        if let Some(sound) = &self.sound {
            sound.chime();
        }
    }

    fn sync_window(&mut self, ctx: &egui::Context) {
        if self.prev_minimized != self.minimized {
            self.prev_minimized = self.minimized;
            if self.minimized {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::InnerSize(egui::vec2(1.0, 1.0)),
                );
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::OuterPosition(egui::pos2(-32000.0, -32000.0)),
                );
            } else {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::InnerSize(egui::vec2(WIDGET_W, WIDGET_H)),
                );
                let level = if self.settings.always_on_top {
                    egui::WindowLevel::AlwaysOnTop
                } else {
                    egui::WindowLevel::Normal
                };
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::WindowLevel(level),
                );
            }
        }
        if !self.placed && !self.minimized {
            if let Some(mon) = ctx.input(|i| i.viewport().monitor_size) {
                self.placed = true;
                let x = mon.x - WIDGET_W - 24.0;
                let y = mon.y - WIDGET_H - 72.0;
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::OuterPosition(egui::pos2(x, y)),
                );
            }
        }
    }

    fn root_close_requested(&mut self, ctx: &egui::Context) {
        if self.quitting {
            return;
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd_to(
                egui::ViewportId::ROOT,
                egui::ViewportCommand::CancelClose,
            );
            if self.overlay_open {
                self.overlay_open = false;
            } else {
                self.minimized = true;
            }
        }
    }

    fn widget_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let frame = egui::Frame::default()
            .fill(PANEL_FILL)
            .inner_margin(egui::Margin::same(9));
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            ui.set_min_size(egui::vec2(WIDGET_W - 18.0, WIDGET_H - 18.0));

            let drag_rect = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 18.0),
                egui::Sense::drag(),
            );
            if drag_rect.1.drag_started() {
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::ROOT,
                    egui::ViewportCommand::StartDrag,
                );
            }
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("WalkUpie")
                        .size(11.0)
                        .color(egui::Color32::from_gray(140)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, BtnIcon::HideTray, "Minimize to tray", 16.0) {
                        self.minimized = true;
                    }
                });
            });

            let phase_color = match self.timer.phase {
                Phase::Work => WORK_COLOR,
                Phase::Break => BREAK_COLOR,
            };

            ui.add_space(4.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(fmt_mmss(self.timer.remaining()))
                        .size(42.0)
                        .strong()
                        .monospace()
                        .color(phase_color),
                );
                let status = if self.timer.is_paused() {
                    "paused"
                } else {
                    "focus"
                };
                ui.label(
                    egui::RichText::new(format!("{} · {}", self.timer.phase.label(), status))
                        .size(12.0)
                        .color(TEXT_COLOR),
                );
            });

            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let pause_icon = if self.timer.is_paused() {
                        BtnIcon::Play
                    } else {
                        BtnIcon::Pause
                    };
                    let pause_tip = if self.timer.is_paused() {
                        "Resume"
                    } else {
                        "Pause"
                    };
                    if icon_button(ui, pause_icon, pause_tip, 26.0) {
                        self.timer.toggle_pause();
                    }
                    if icon_button(ui, BtnIcon::Skip, "Skip to next phase", 26.0) {
                        let t = self.timer.skip();
                        self.apply_transition(t);
                    }
                    if icon_button(ui, BtnIcon::Gear, "Settings", 26.0) {
                        self.settings_open = true;
                        self.draft = self.settings.clone();
                    }
                });
            });
        });
    }

    fn apply_transition(&mut self, transition: Transition) {
        match transition {
            Transition::ToBreak => {
                self.overlay_open = true;
                self.chime();
                self.persist();
            }
            Transition::ToWork => {
                self.overlay_open = false;
                self.chime();
                self.persist();
            }
        }
    }

    fn overlay_ui(&mut self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("walkupie-overlay"),
            egui::ViewportBuilder::default()
                .with_title("Time to walk!")
                .with_decorations(false)
                .with_resizable(false)
                .with_always_on_top()
                .with_fullscreen(true)
                .with_taskbar(false),
            |ui, _class| {
                let phase_color = match self.timer.phase {
                    Phase::Work => WORK_COLOR,
                    Phase::Break => BREAK_COLOR,
                };
                let frame = egui::Frame::default()
                    .fill(egui::Color32::from_rgb(16, 18, 24))
                    .inner_margin(egui::Margin::symmetric(48, 48));
                egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() * 0.14);
                        ui.label(
                            egui::RichText::new("Time to get up!")
                                .size(44.0)
                                .strong()
                                .color(egui::Color32::from_gray(240)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "You've been working — walk for {} minutes.",
                                self.timer.duration().as_secs() / 60
                            ))
                            .size(20.0)
                            .color(egui::Color32::from_gray(190)),
                        );
                        ui.add_space(28.0);
                        ui.label(
                            egui::RichText::new(fmt_mmss(self.timer.remaining()))
                                .size(96.0)
                                .strong()
                                .monospace()
                                .color(phase_color),
                        );
                        ui.add_space(32.0);
                        ui.horizontal(|ui| {
                            let snooze = egui::Button::new("Snooze 5 min")
                                .min_size(egui::vec2(190.0, 46.0));
                            if ui.add(snooze).clicked() {
                                self.timer.snooze(Duration::from_secs(5 * 60));
                                self.chime();
                            }
                            let done =
                                egui::Button::new("I'm back").min_size(egui::vec2(190.0, 46.0));
                            if ui.add(done).clicked() {
                                self.overlay_open = false;
                                let t = self.timer.skip();
                                self.apply_transition(t);
                            }
                        });
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new("Snooze restarts the break and plays a chime")
                                .size(13.0)
                                .color(egui::Color32::from_gray(120)),
                        );
                    });
                    if ui.input(|i| i.viewport().close_requested()) {
                        self.overlay_open = false;
                        let t = self.timer.skip();
                        self.apply_transition(t);
                    }
                });
            },
        );
    }

    fn settings_ui(&mut self, ctx: &egui::Context) {
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("walkupie-settings"),
            egui::ViewportBuilder::default()
                .with_title("WalkUpie Settings")
                .with_inner_size([300.0, 250.0])
                .with_resizable(false),
            |ui, _class| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.draft.work_minutes, 1..=240)
                            .text("Work (minutes)"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.draft.break_minutes, 1..=120)
                            .text("Break (minutes)"),
                    );
                    ui.add_space(6.0);
                    ui.checkbox(&mut self.draft.always_on_top, "Always on top");
                    ui.checkbox(&mut self.draft.auto_start, "Start with Windows");
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.save_settings(ctx);
                        }
                        if ui.button("Cancel").clicked() {
                            self.settings_open = false;
                        }
                    });
                });
            },
        );
    }
}

impl eframe::App for WalkUpie {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray(ctx);

        if let Some(t) = self.timer.tick() {
            self.apply_transition(t);
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.root_close_requested(&ctx);
        self.sync_window(&ctx);

        if !self.minimized {
            self.widget_ui(ui);
        }

        if self.settings_open {
            self.settings_ui(&ctx);
        }
        if self.overlay_open {
            self.overlay_ui(&ctx);
        }

        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist();
    }
}
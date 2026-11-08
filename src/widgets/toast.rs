use eframe::egui::{self, Color32, Id, Order, RichText, Stroke};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    fn color(&self) -> Color32 {
        match self {
            ToastLevel::Info => Color32::from_rgb(52, 152, 219),
            ToastLevel::Success => Color32::from_rgb(46, 204, 113),
            ToastLevel::Warning => Color32::from_rgb(243, 156, 18),
            ToastLevel::Error => Color32::from_rgb(231, 76, 60),
        }
    }
}

pub struct Toast {
    pub id: u64,
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub duration: Duration,
    pub closing: bool,
    pub enter_start_time: Instant,
    pub close_start_time: Option<Instant>,
}

pub struct ToastManager {
    toasts: Vec<Toast>,
    next_id: u64,
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            next_id: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    pub fn push(&mut self, level: ToastLevel, message: impl Into<String>) {
        // 限制最多 2 个，超出强制最早的滑出
        if self.toasts.len() >= 2 {
            if let Some(t) = self.toasts.iter_mut().find(|t| !t.closing) {
                t.closing = true;
                t.close_start_time = Some(Instant::now());
            }
        }

        self.next_id += 1;
        let now = Instant::now();
        self.toasts.push(Toast {
            id: self.next_id,
            message: message.into(),
            level,
            created_at: now,
            duration: Duration::from_secs(3),
            closing: false,
            enter_start_time: now,
            close_start_time: None,
        });
    }

    pub fn render(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.content_rect();
        let now = Instant::now();

        let toast_width = 220.0;
        let toast_height = 48.0;
        let margin = 20.0;
        let spacing = 10.0;

        let enter_time = 0.35;
        // 干脆利落的 0.2 秒滑出
        let exit_time = 0.2;

        let mut to_remove = Vec::new();

        // 正常完全显示时，弹窗的左侧 X 坐标
        let target_x = screen_rect.right() - toast_width - margin;
        // 完全隐藏时，弹窗的左侧 X 坐标（刚好越过屏幕右侧边界）
        let offscreen_x = screen_rect.right() + 10.0;

        for (i, toast) in self.toasts.iter_mut().enumerate() {
            if !toast.closing && now.duration_since(toast.created_at) > toast.duration {
                toast.closing = true;
                toast.close_start_time = Some(now);
            }

            let current_x = if toast.closing {
                let t_exit = (now - toast.close_start_time.unwrap()).as_secs_f32() / exit_time;

                if t_exit >= 1.0 {
                    to_remove.push(i);
                    continue;
                }

                // 改用二次方加速，让它是“飞”出去的，而不是匀速平移
                let t = t_exit * t_exit;
                target_x + (offscreen_x - target_x) * t
            } else {
                let t_enter = (now - toast.enter_start_time).as_secs_f32() / enter_time;
                // 滑入保持回弹感
                let t = ease_out_back(t_enter.clamp(0.0, 1.0));
                offscreen_x + (target_x - offscreen_x) * t
            };

            // Y 轴丝滑补位
            let target_y = screen_rect.bottom() - margin - (toast_height + spacing) * i as f32 - toast_height;
            let current_y = ctx.animate_value_with_time(
                Id::unique(("toast_y", toast.id)),
                target_y,
                0.2,
            );

            // 滑出时，最后半段开始淡出
            let opacity = if toast.closing {
                let t_exit = (now - toast.close_start_time.unwrap()).as_secs_f32() / exit_time;
                // 前 50% 的时间透明度为 1，后 50% 的时间透明度从 1 降到 0
                (1.0 - (t_exit - 0.5).max(0.0) * 2.0).clamp(0.0, 1.0)
            } else {
                let t_enter = (now - toast.enter_start_time).as_secs_f32() / enter_time;
                t_enter.clamp(0.0, 1.0)
            };

            let pos = egui::pos2(current_x, current_y);
            let level_color = toast.level.color();
            let area_id = Id::unique(("toast_area", toast.id));

            let response = egui::Area::new(area_id)
                .order(Order::Foreground)
                .fixed_pos(pos)
                .constrain(false) // 允许飞出窗口边缘
                .show(ctx, |ui| {
                    let is_dark = ui.visuals().dark_mode;
                    let bg_color = if is_dark {
                        Color32::from_rgb(45, 45, 48).gamma_multiply(opacity)
                    } else {
                        Color32::from_rgb(255, 255, 255).gamma_multiply(opacity)
                    };
                    let text_color = if is_dark { Color32::WHITE } else { Color32::BLACK }.gamma_multiply(opacity);

                    let frame = egui::Frame::default()
                        .fill(bg_color)
                        .stroke(Stroke::new(1.0, level_color.gamma_multiply(opacity)))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 2],
                            blur: 8,
                            spread: 0,
                            color: Color32::from_black_alpha((30.0 * opacity) as u8),
                        });

                    frame.show(ui, |ui| {
                        ui.set_width(toast_width - 20.0);
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, level_color.gamma_multiply(opacity));
                            ui.add_space(4.0);
                            ui.label(RichText::new(&toast.message).color(text_color).size(13.0));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let btn = egui::Button::new(RichText::new("X").size(12.0).color(text_color)).frame(false);
                                if ui.add(btn).clicked() {
                                    if !toast.closing {
                                        toast.closing = true;
                                        toast.close_start_time = Some(now);
                                    }
                                }
                            });
                        });
                    });
                });

            // 悬停暂停
            if response.response.contains_pointer() && !toast.closing {
                let t_enter = (now - toast.enter_start_time).as_secs_f32() / enter_time;
                if t_enter >= 1.0 {
                    toast.created_at = now;
                }
            }
        }

        for &i in to_remove.iter().rev() {
            self.toasts.remove(i);
        }

        if !self.toasts.is_empty() {
            // 只要有弹窗，强制 16ms 刷新一次（60fps），防止卡顿
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
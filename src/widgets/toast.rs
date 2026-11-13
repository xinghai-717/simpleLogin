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
        let now = Instant::now();

        let margin = 20.0;
        let spacing = 10.0;
        let enter_time = 0.35;
        let exit_time = 0.2;

        let mut to_remove = Vec::new();

        for (i, toast) in self.toasts.iter_mut().enumerate() {
            if !toast.closing && now.duration_since(toast.created_at) > toast.duration {
                toast.closing = true;
                toast.close_start_time = Some(now);
            }

            // 计算 X 轴滑动偏移量 (基于锚点的相对距离)
            // 正常显示时，距离右边缘 -margin
            let target_x_offset = -margin;
            // 滑出屏幕外时，向右偏移 400 像素
            let offscreen_x_offset = 400.0;

            let current_offset_x = if toast.closing {
                let t_exit = (now - toast.close_start_time.unwrap()).as_secs_f32() / exit_time;
                if t_exit >= 1.0 {
                    to_remove.push(i);
                    continue;
                }
                let t = t_exit * t_exit;
                target_x_offset + (offscreen_x_offset - target_x_offset) * t
            } else {
                let t_enter = (now - toast.enter_start_time).as_secs_f32() / enter_time;
                let t = ease_out_back(t_enter.clamp(0.0, 1.0));
                offscreen_x_offset + (target_x_offset - offscreen_x_offset) * t
            };

            // 计算 Y 轴堆叠偏移量
            let toast_height = 48.0;
            let target_y_offset = -margin - (toast_height + spacing) * i as f32;
            let current_y_offset = ctx.animate_value_with_time(
                Id::unique(("toast_y", toast.id)),
                target_y_offset,
                0.2,
            );

            // 透明度动画
            let opacity = if toast.closing {
                let t_exit = (now - toast.close_start_time.unwrap()).as_secs_f32() / exit_time;
                (1.0 - (t_exit - 0.5).max(0.0) * 2.0).clamp(0.0, 1.0)
            } else {
                let t_enter = (now - toast.enter_start_time).as_secs_f32() / enter_time;
                t_enter.clamp(0.0, 1.0)
            };

            let level_color = toast.level.color();
            let area_id = Id::unique(("toast_area", toast.id));

            // 精确测量文本宽度
            let painter = ctx.layer_painter(egui::LayerId::new(Order::Background, Id::unique("toast_measure")));
            let galley = painter.layout_no_wrap(
                toast.message.clone(),
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
            let text_width = galley.size().x;

            // 根据文字长度计算弹窗总宽度
            // 基础宽度 60.0 = 左侧竖条(4) + 间距(8) + 右侧X按钮(28) + 左右内边距(20)
            // 限制最小 120，最大 400（超过 400 就会截断）
            let total_width = (text_width + 60.0).clamp(120.0, 400.0);

            let response = egui::Area::new(area_id)
                .order(Order::Foreground)
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(current_offset_x, current_y_offset))
                .constrain(false)
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
                        // 强制限制整个 UI 的宽度
                        ui.set_width(total_width - 20.0); // 减去 Frame 的左右 10px 内边距

                        ui.horizontal(|ui| {
                            // 左侧竖条
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(4.0, 16.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, level_color.gamma_multiply(opacity));
                            ui.add_space(4.0);

                            // 给文字精确分配空间，逼迫它截断
                            let available_for_text = ui.available_width() - 24.0; // 留出 24px 给右侧按钮
                            ui.allocate_ui_with_layout(
                                egui::vec2(available_for_text, ui.available_height()),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(RichText::new(&toast.message).color(text_color).size(13.0))
                                            .truncate()
                                    );
                                }
                            );

                            // 右侧关闭按钮
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
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
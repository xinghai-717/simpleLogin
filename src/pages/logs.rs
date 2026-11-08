use eframe::egui::{self, Color32, FontId, TextFormat};
use crate::logger::LogBuffer;

#[derive(Default)]
pub struct LogPage;

impl LogPage {
    pub fn ui(&mut self, ui: &mut egui::Ui, log_buffer: &LogBuffer) {
        ui.horizontal(|ui| {
            ui.heading("运行日志");

            // 右侧操作按钮
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("清空").clicked() {
                    log_buffer.lock().unwrap().clear();
                }
                if ui.button("复制全部").clicked() {
                    let logs = log_buffer.lock().unwrap();
                    let text = logs.iter().cloned().collect::<Vec<_>>().join("");
                    // 复制时需要去除 ANSI 转义码，否则粘贴出来是乱码
                    let clean_text = strip_ansi(&text);
                    ui.ctx().copy_text(clean_text);
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let logs: Vec<String> = {
                    let buffer = log_buffer.lock().unwrap();
                    buffer.iter().cloned().collect()
                };

                let font_id = FontId::monospace(12.0);

                for line in &logs {
                    let mut job = egui::text::LayoutJob::default();
                    ansi_to_layout_job(line, &mut job, &font_id, ui.visuals().text_color());
                    ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Extend));
                }
            });
    }
}

/// 简易 ANSI 转 LayoutJob 解析器
fn ansi_to_layout_job(text: &str, job: &mut egui::text::LayoutJob, font_id: &FontId, default_color: Color32) {
    let mut current_color = default_color;
    let mut current_text = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // 遇到转义符，先把之前的文本写进 Job
            if !current_text.is_empty() {
                job.append(
                    &current_text,
                    0.0,
                    TextFormat { font_id: font_id.clone(), color: current_color, ..Default::default() },
                );
                current_text.clear();
            }

            // 解析 ANSI 序列 \x1b[...m
            if chars.peek() == Some(&'[') {
                chars.next(); // 消耗 '['
                let mut code = String::new();
                for cc in chars.by_ref() {
                    if cc == 'm' {
                        break;
                    }
                    code.push(cc);
                }

                // 简单的颜色映射
                current_color = match code.as_str() {
                    "30" => Color32::DARK_GRAY,
                    "31" => Color32::from_rgb(231, 76, 60),  // 红 (Error)
                    "32" => Color32::from_rgb(46, 204, 113), // 绿 (Success)
                    "33" => Color32::from_rgb(243, 156, 18), // 黄 (Warn)
                    "34" => Color32::from_rgb(52, 152, 219), // 蓝
                    "35" => Color32::from_rgb(155, 89, 182), // 紫
                    "36" => Color32::from_rgb(26, 188, 156), // 青
                    "0" | "" => default_color,               // 重置
                    _ => current_color,                      // 其他代码忽略
                };
            }
        } else {
            // 普通字符，直接收集
            current_text.push(c);
        }
    }

    // 把最后一段文本写进去
    if !current_text.is_empty() {
        job.append(
            &current_text,
            0.0,
            TextFormat { font_id: font_id.clone(), color: current_color, ..Default::default() },
        );
    }
}

/// 去除 ANSI 转义码，用于复制到剪贴板
fn strip_ansi(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for cc in chars.by_ref() {
                    if cc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}
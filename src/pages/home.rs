use eframe::egui;

/// 主页自己的状态。将来可以装登录表单、状态显示等。
#[derive(Default)]
pub struct HomePage {
    pub username: String,
    pub password: String,
    pub status_text: String,
}

impl HomePage {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::Label::new(egui::RichText::new("主页").heading()).selectable(false),
        );
        ui.separator();

        egui::Grid::new("login_grid").num_columns(2).show(ui, |ui| {
            ui.label("账号:");
            ui.text_edit_singleline(&mut self.username);
            ui.end_row();

            ui.label("密码:");
            ui.text_edit_singleline(&mut self.password);
            ui.end_row();
        });

        if ui.button("登录").clicked() {
            self.status_text = format!("登录中... (账号 {})", self.username);
            // TODO
        }

        if !self.status_text.is_empty() {
            ui.separator();
            ui.label(&self.status_text);
        }
    }
}
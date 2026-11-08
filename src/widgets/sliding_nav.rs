use eframe::egui;

/// 纵向滑动高亮导航。每个选项竖直排列，高亮块沿 Y 轴滑动。
pub fn sliding_nav<T>(
    ui: &mut egui::Ui,
    id_salt: &str,
    current: &mut T,
    options: &[(T, &str)],
    item_width: f32,
    item_height: f32,
    item_spacing: f32,
) where
    T: PartialEq + Copy,
{
    let mut rects: Vec<(T, egui::Rect)> = Vec::with_capacity(options.len());
    let mut clicked_idx = None; // 记录是否发生了点击

    // 1) 渲染选项
    for (idx, &(value, label)) in options.iter().enumerate() {
        let is_selected = *current == value;
        let response = ui.add_sized(
            [item_width, item_height],
            egui::Button::selectable(is_selected, label).frame(false),
        );
        rects.push((value, response.rect));

        if response.clicked() {
            *current = value;
            clicked_idx = Some(idx); // 记录点击的索引
        }
        ui.add_space(item_spacing);
    }

    // 2) 找到当前选中项的 Rect
    let Some((_, target_rect)) = rects.iter().find(|(v, _)| *v == *current) else {
        return;
    };

    let anim_id = ui.scope_id().with(id_salt);

    // 3) 如果发生了点击，计算距离并更新动画时长
    if let Some(new_idx) = clicked_idx {
        // 读取上一帧的索引
        let old_idx = ui.data(|d| d.get_temp::<usize>(anim_id.with("prev_idx"))).unwrap_or(new_idx);
        let distance = (new_idx as isize - old_idx as isize).abs() as f32;

        // 基准 0.1s，跨一个选项加 0.05s，最长 0.3s
        let anim_time = (0.1 + distance * 0.05).min(0.3);

        // 写入新的索引和对应的动画时长
        ui.data_mut(|d| d.insert_temp(anim_id.with("prev_idx"), new_idx));
        ui.data_mut(|d| d.insert_temp(anim_id.with("anim_time"), anim_time));
    }

    // 4) 读取动画时长（如果没点过，默认 0.1s）
    let anim_time = ui.data(|d| d.get_temp::<f32>(anim_id.with("anim_time"))).unwrap_or(0.1);

    // 5) 执行动画
    let current_y = ui.ctx().animate_value_with_time(
        anim_id.with("indicator_y"),
        target_rect.top(),
        anim_time,
    );

    // 6) 画高亮块
    let indicator_rect = egui::Rect::from_min_size(
        egui::pos2(target_rect.left(), current_y),
        egui::vec2(target_rect.width(), target_rect.height()),
    );
    ui.painter().rect_filled(
        indicator_rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(100, 150, 255, 80),
    );
}
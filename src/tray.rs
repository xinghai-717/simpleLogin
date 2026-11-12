use image::GenericImageView;
use tray_icon::{
    menu::{Menu, MenuItem, MenuEvent},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub struct TrayApp {
    pub tray_icon: TrayIcon,
    pub quit_item: MenuItem,
    pub show_item: MenuItem,
}

impl TrayApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let icon_bytes = include_bytes!("../assets/app.ico"); // 确保路径正确
        let img = image::load_from_memory(icon_bytes)?;
        let (width, height) = img.dimensions();
        let icon = tray_icon::Icon::from_rgba(img.to_rgba8().into_raw(), width, height)?;

        let menu = Menu::new();
        let show_item = MenuItem::new("显示主界面", true, None);
        let quit_item = MenuItem::new("退出", true, None);
        menu.append(&show_item)?;
        menu.append(&quit_item)?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("SimpleLogin")
            .with_icon(icon)
            .build()?;

        Ok(Self {
            tray_icon,
            quit_item,
            show_item,
        })
    }
}
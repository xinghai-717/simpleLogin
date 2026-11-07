pub mod home;
pub mod settings;
pub mod about;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Home,
    Settings,
    About,
}

impl Page {
    pub const ALL: &[(Page, &str)] = &[
        (Page::Home, "主页"),
        (Page::Settings, "设置"),
        (Page::About, "关于"),
    ];
}
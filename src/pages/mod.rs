pub mod home;
pub mod account;
pub mod settings;
pub mod about;
pub mod logs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    Home,
    #[default]
    Account,
    Settings,
    Logs,
    About,
}

impl Page {
    pub const ALL: &[(Page, &str)] = &[
        (Page::Home, "主页"),
        (Page::Account, "账号"),
        (Page::Settings, "设置"),
        (Page::Logs, "日志"),
        (Page::About, "关于"),
    ];
}
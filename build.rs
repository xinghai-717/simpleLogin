fn main() {
    // 仅在 Windows 系统下编译资源，避免在其他平台上报错
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.compile().expect("Windows 资源编译失败");
    }
}
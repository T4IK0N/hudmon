fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    #[cfg(windows)]
    {
        if std::path::Path::new("assets/icon.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            res.set("FileDescription", "hudmon - overlay FPS/CPU/GPU/RAM");
            if let Err(e) = res.compile() {
                println!("cargo:warning=nie udalo sie osadzic ikony: {e}");
            }
        }
    }
}

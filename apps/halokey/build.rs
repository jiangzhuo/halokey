fn main() {
    // 前端产物变了也要重新编译：generate_context! 在编译期把 ui/dist 打进二进制，
    // 而 tauri_build 本身不跟踪这个目录。
    println!("cargo:rerun-if-changed=../../ui/dist");
    // 默认特性 custom-protocol 要嵌入 ui/dist；目录不存在时 tauri 的 panic 很难读，这里先给出人话。
    if cfg!(feature = "custom-protocol")
        && !std::path::Path::new("../../ui/dist/index.html").exists()
    {
        panic!(
            "ui/dist 不存在：先 `cd ui && npm install && npm run build`，\
             或者开发时用 `cargo run -p halokey-app --no-default-features`（加载 Vite 开发服务器）。"
        );
    }
    tauri_build::build()
}

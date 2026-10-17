fn main() {
    // 前端产物变了也要重新编译：generate_context! 在编译期把 ui/dist 打进二进制，
    // 而 tauri_build 本身不跟踪这个目录。
    println!("cargo:rerun-if-changed=../../ui/dist");
    tauri_build::build()
}

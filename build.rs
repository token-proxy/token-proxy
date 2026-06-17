use std::fs;

fn main() {
    // rust-embed 编译期需要 dist/ 目录存在。npm run build 会产出真实前端文件，
    // 但 cargo check / clippy 可能在 npm build 之前运行。
    // 此处仅在 dist/index.html 不存在时创建一个最小占位，避免编译报错。
    if !fs::exists("dist/index.html").unwrap_or(false) {
        fs::create_dir_all("dist").ok();
        fs::write(
            "dist/index.html",
            "<!DOCTYPE html><html><body></body></html>",
        )
        .ok();
    }
}

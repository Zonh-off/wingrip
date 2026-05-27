fn main() {
    // Compile and embed the resource script for Windows target
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        embed_resource::compile("wingrip.rc", embed_resource::NONE);
    }
}

// I hate that this is needed...
fn main() {
    println!("cargo::rustc-check-cfg=cfg(loom)");
}

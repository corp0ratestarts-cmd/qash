// Two-stage build pipeline placeholder
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-env=QASH_BUILD_STAGE=1");
    println!("cargo:rustc-env=PLATFORM_MEASUREMENT=0x0000000000000000000000000000000000000000000000000000000000000000");
}

fn main() {
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=MC_BUILD_TARGET={target}");
    println!("cargo:rustc-env=MC_BUILD_PROFILE={profile}");
    println!("cargo:rerun-if-changed=build.rs");
}

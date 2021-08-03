fn main() {
    if version_check::Channel::read()
        .map(|c| c.is_nightly())
        .unwrap_or(false)
    {
        println!("cargo:rustc-cfg=nightly");
    }
}

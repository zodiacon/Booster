fn main() -> Result<(), wdk_build::ConfigError> {
    std::env::set_var("CARGO_CFG_TARGET_FEATURE", "crt-static");
    wdk_build::configure_wdk_binary_build()
}

fn main() -> Result<(), String> {
    unsafe {
        fs_common::game::client::render::BUILD_DATETIME = option_env!("BUILD_DATETIME");
        fs_common::game::client::render::GIT_HASH = option_env!("GIT_HASH");
    }

    fs_common::main()
}

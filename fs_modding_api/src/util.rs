mod _impl {
    wasm_plugin_guest::import_functions! {
        fn get_time() -> std::time::SystemTime;
    }

    pub fn pub_get_time() -> std::time::SystemTime {
        get_time()
    }
}

pub use _impl::pub_get_time as get_time;

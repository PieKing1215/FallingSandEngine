use log::{Level, Metadata, Record};

pub(crate) struct FSModLogger;

impl log::Log for FSModLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                Level::Debug => log_debug(format!("{}", record.args())),
                Level::Info => log_info(format!("{}", record.args())),
                Level::Warn => log_warn(format!("{}", record.args())),
                Level::Error => log_error(format!("{}", record.args())),
                _ => {},
            }
        }
    }

    fn flush(&self) {}
}

wasm_plugin_guest::import_functions! {
    fn log_debug(msg: String);
    fn log_info(msg: String);
    fn log_warn(msg: String);
    fn log_error(msg: String);
}

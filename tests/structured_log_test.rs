
extern crate log;

extern crate serde_json;

use log::{Record, Metadata, Log};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use pogr_log_rs::structured_log;
use log::set_logger;
use serde_json::json;


struct TestLogger {
    pub messages: Mutex<Vec<String>>, // Stores log messages
}

impl Log for TestLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true // Capture all log levels
    }

    fn log(&self, record: &Record) {
        let message = format!("{}", record.args());
        self.messages.lock().unwrap().push(message);
    }

    fn flush(&self) {}
}

static TEST_LOGGER: Lazy<TestLogger> = Lazy::new(|| {
    TestLogger {
        messages: Mutex::new(vec![]),
    }
});

fn init_test_logger() {
    set_logger(unsafe { &*(&*TEST_LOGGER as *const TestLogger as *const dyn Log) }).unwrap();
    log::set_max_level(log::LevelFilter::Trace); // Adjust as needed
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_log_macro() {
        init_test_logger();

        // Use the structured_log! macro
        structured_log!(
            log::Level::Info,
            "User logged in",
            "login",
            json!({"user_id": 123}), // Correctly use the json! macro here
            json!({"env": "production"}) // And here
        );
        // Check the captured log message
        let messages = &TEST_LOGGER.messages.lock().unwrap();
        assert_eq!(messages.len(), 1);

        // Parse the captured log message as JSON to verify its structure
        let log_message: serde_json::Value = serde_json::from_str(&messages[0]).expect("Failed to parse log message as JSON");
        assert_eq!(log_message["log"], "User logged in");
        assert_eq!(log_message["type"], "login");
        assert_eq!(log_message["data"]["user_id"], 123);
        assert_eq!(log_message["tags"]["env"], "production");
    }
}
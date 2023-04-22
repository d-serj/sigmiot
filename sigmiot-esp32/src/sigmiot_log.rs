use std::sync::{Mutex, Arc};
use esp_idf_hal::task::embassy_sync::EspRawMutex;
use log::{error, Log, Record, Level, Metadata, info};
use esp_idf_svc::log::EspLogger;
use lazy_static::lazy_static;

#[derive(Debug)]
pub struct RemoteLoggerEntry {
    pub level: String,
    pub target: String,
    pub message: String,
    pub timestamp: u64,
}

pub struct RemoteLogger {
    enabled: bool,
}

pub struct MultiLogger {
    esp_logger: &'static EspLogger,
    remote_logger: &'static Mutex<RemoteLogger>,
}

const LOG_CHANNEL_SIZE: usize = 21;
static LOG_CHANNEL: embassy_sync::channel::Channel<EspRawMutex, RemoteLoggerEntry, LOG_CHANNEL_SIZE> =
    embassy_sync::channel::Channel::new();

impl Log for MultiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.esp_logger.enabled(metadata) || self.remote_logger.lock().unwrap().enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.esp_logger.enabled(record.metadata()) {
            self.esp_logger.log(record);
        }

        if self.remote_logger.lock().unwrap().enabled(record.metadata()) {
            // Only log info and above to remote logger
            //if record.level() <= Level::Warn {
                self.remote_logger.lock().unwrap().log(record);
            //}
        }
    }

    fn flush(&self) {
        self.esp_logger.flush();
        self.remote_logger.lock().unwrap().flush();
    }
}

impl RemoteLogger {
    pub fn new() -> Self {
        Self { enabled: false, }
    }

    fn enabled(&self, metadata: &Metadata) -> bool {
        self.enabled
    }

    pub fn set_enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn log(&self, record: &Record) {
        let entry = RemoteLoggerEntry {
            level: record.level().as_str().to_string(),
            target: record.target().to_string(),
            message: record.args().to_string(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };

        remote_logger_publish_entry(entry);
    }

    fn flush(&self) {
        while let Ok(_) = LOG_CHANNEL.try_recv() {}
    }
}

fn remote_logger_publish_entry(entry: RemoteLoggerEntry) {
    if let Err(e) = LOG_CHANNEL.try_send(entry) {
        error!("Failed to send log entry to remote logger: {:?}", e);
    }
}

pub fn remote_logger_get_entries() -> Vec<RemoteLoggerEntry> {
    let mut entries = Vec::new();

    while let Ok(entry) = LOG_CHANNEL.try_recv() {
        entries.push(entry);
    }

    entries
}

static LOGGER: EspLogger = EspLogger;

lazy_static!{
    static ref REMOTE_LOGGER: Mutex<RemoteLogger> = Mutex::new(RemoteLogger::new());
}

pub fn sigmiot_log_init() {
    let multi_log = Box::new(MultiLogger {
        esp_logger: &LOGGER,
        remote_logger: &REMOTE_LOGGER,
    });

    log::set_boxed_logger(multi_log).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}

pub fn remote_logger_set_enable(enabled: bool) {
    if enabled {
        info!("Remote logger enabled");
    } else {
        info!("Remote logger disabled");
        REMOTE_LOGGER.lock().unwrap().flush();
    }

    REMOTE_LOGGER.lock().unwrap().set_enable(enabled);
}

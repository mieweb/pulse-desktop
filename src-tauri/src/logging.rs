use log::LevelFilter;
use std::sync::Mutex;
use std::time::Instant;
use env_logger::{Builder, Target};

/// Global timestamp for delta calculation
static LAST_LOG: Mutex<Option<Instant>> = Mutex::new(None);

/// Initialize the logger with delta timestamps
pub fn init() {
    Builder::new()
        .target(Target::Stdout)
        .format(|buf, record| {
            use std::io::Write;
            
            let now = Instant::now();
            let mut last = LAST_LOG.lock().unwrap();
            let delta = last.map(|t| now.duration_since(t).as_millis()).unwrap_or(0);
            *last = Some(now);

            writeln!(
                buf,
                "{} [+{} ms] [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                delta,
                record.level(),
                record.args()
            )
        })
        .filter_level(LevelFilter::Debug)
        .init();
}

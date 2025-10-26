use log::LevelFilter;
use std::sync::Mutex;
use std::time::Instant;
use env_logger::{Builder, Target};
use chrono::{Local, NaiveDate};

/// Global timestamp for delta calculation
static LAST_LOG: Mutex<Option<Instant>> = Mutex::new(None);

/// Global last date for date change detection
static LAST_DATE: Mutex<Option<NaiveDate>> = Mutex::new(None);

/// Initialize the logger with delta timestamps and smart date formatting
pub fn init() {
    Builder::new()
        .target(Target::Stdout)
        .format(|buf, record| {
            use std::io::Write;
            
            let now = Instant::now();
            let mut last = LAST_LOG.lock().unwrap();
            let delta = last.map(|t| now.duration_since(t).as_millis()).unwrap_or(0);
            *last = Some(now);

            let now_datetime = Local::now();
            let current_date = now_datetime.date_naive();
            
            // Check if we need to show the date
            let mut last_date = LAST_DATE.lock().unwrap();
            let show_date = match *last_date {
                None => true,  // First log, show date
                Some(prev_date) => prev_date != current_date,  // Date changed, show it
            };
            *last_date = Some(current_date);

            if show_date {
                // Show full date and time
                writeln!(
                    buf,
                    "{} [+{} ms] [{}] - {}",
                    now_datetime.format("%Y-%m-%d %H:%M:%S%.3f"),
                    delta,
                    record.level(),
                    record.args()
                )
            } else {
                // Show only time
                writeln!(
                    buf,
                    "{} [+{} ms] [{}] - {}",
                    now_datetime.format("%H:%M:%S%.3f"),
                    delta,
                    record.level(),
                    record.args()
                )
            }
        })
        .filter_level(LevelFilter::Debug)
        .init();
}

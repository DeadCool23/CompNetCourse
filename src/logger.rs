use ansi_term::Colour;
use chrono_tz::Europe::Moscow;
use env_logger::Builder;
use std::{fs::OpenOptions, io::Write};

pub fn init() {
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("server.log")
        .expect(&format!("Can't open server.log"));

    Builder::from_default_env()
        .format(move |buf, record| {
            let timestamp = chrono::Utc::now()
                .with_timezone(&Moscow)
                .format("%Y-%m-%dT%H:%M:%SZ%:z");

            let level = match record.level() {
                log::Level::Error => Colour::Red.paint(record.level().to_string()),
                log::Level::Warn => Colour::Yellow.paint(record.level().to_string()),
                log::Level::Info => Colour::Green.paint(record.level().to_string()),
                log::Level::Debug => Colour::Blue.paint(record.level().to_string()),
                log::Level::Trace => Colour::Purple.paint(record.level().to_string()),
            };

            let log_line = format!(
                "[{} {} {}] {}",
                timestamp,
                level,
                record.module_path().unwrap_or_default(),
                record.args()
            );

            writeln!(
                &log_file,
                "[{} {} {}] {}",
                timestamp,
                record.level(),
                record.module_path().unwrap_or_default(),
                record.args()
            )
            .expect("Failed to write to log file");

            writeln!(buf, "{}", log_line)?;

            Ok(())
        })
        .init();
}

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct ServerConfig {
    /// Хост сервера
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Порт сервера
    #[arg(short, long, default_value_t = 9898)]
    pub port: u16,

    /// Количество рабочих потоков в пуле потоков
    #[arg(short, long, default_value_t = 10)]
    pub threads: usize,

    /// Корневая директория с документами
    #[arg(short, long, default_value = "./static")]
    pub document_root: PathBuf,

    /// Максимальное количество одновременных соединений
    #[arg(long, default_value_t = 1000)]
    pub max_connections: usize,

    /// Максимальный размер файла в байтах (по умолчанию: 128 МБ)
    #[arg(long, default_value_t = 134217728)] // 128 * 1024 * 1024
    pub max_file_size: u64,

    /// Таймаут pselect в секундах
    #[arg(long, default_value_t = 1)]
    pub select_timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9898,
            threads: 10,
            document_root: PathBuf::from("./static"),
            max_connections: 1000,
            max_file_size: 134217728,
            select_timeout: 1
        }
    }
}
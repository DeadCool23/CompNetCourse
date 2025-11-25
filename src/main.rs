mod logger;
mod server;
mod static_files;

use clap::Parser;
use log::info;
use server::HttpServer;
use server::config::ServerConfig;

fn main() -> std::io::Result<()> {
    logger::init();

    let config = ServerConfig::parse();
    info!("Starting Static HTTP Server with config: {:?}", config);

    let server = HttpServer::new(&config)?;
    server.run();

    Ok(())
}

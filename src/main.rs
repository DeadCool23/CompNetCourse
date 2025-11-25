mod server;
mod static_files;

use clap::Parser;
use log::info;
use server::config::ServerConfig;
use server::HttpServer;

fn main() -> std::io::Result<()> {
    env_logger::init();
    
    let config = ServerConfig::parse();
    info!("Starting Static HTTP Server with config: {:?}", config);
    
    let server = HttpServer::new(&config)?;
    server.run();
    
    Ok(())
}
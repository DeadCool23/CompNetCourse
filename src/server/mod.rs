pub mod config;
pub mod connection;
pub mod connection_handler;
pub mod connection_manager;
pub mod http_status;
pub mod request_parser;
pub mod select_handler;

use log::{debug, error, info, warn};
use std::os::fd::AsRawFd;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use threadpool::ThreadPool;

use config::ServerConfig;
use connection::ConnectionStage;
use connection_handler::ConnectionHandler;
use connection_manager::ConnectionManager;
use request_parser::RequestParser;
use select_handler::SelectHandler;

pub struct HttpServer {
    config: ServerConfig,
    connection_manager: Arc<ConnectionManager>,
    _thread_pool: ThreadPool,
    connection_handler: ConnectionHandler,
    select_handler: SelectHandler,
    parse_rx: Arc<std::sync::Mutex<mpsc::Receiver<(i32, Vec<u8>)>>>,
}

impl HttpServer {
    pub fn new(config: &ServerConfig) -> std::io::Result<Self> {
        let addr = format!("{}:{}", config.host, config.port);
        let listener = std::net::TcpListener::bind(&addr)?;
        listener.set_nonblocking(true)?;

        info!("Server started on {}", addr);

        let connection_manager = Arc::new(ConnectionManager::with_config(listener, config));
        let _thread_pool = ThreadPool::new(config.threads);

        let (parse_tx, parse_rx) = mpsc::channel();
        let parse_rx = Arc::new(std::sync::Mutex::new(parse_rx));

        let request_parser = RequestParser::new(config.document_root.clone(), config.max_file_size);

        let connection_handler =
            ConnectionHandler::new(Arc::clone(&connection_manager), request_parser, parse_tx);

        let select_handler =
            SelectHandler::new(Arc::clone(&connection_manager), config.select_timeout);

        Ok(Self {
            config: config.clone(),
            connection_manager,
            _thread_pool,
            connection_handler,
            select_handler,
            parse_rx,
        })
    }

    pub fn run(&self) {
        info!("Server running with {} threads", self.config.threads);

        if let Err(e) = self.create_default_files() {
            error!("Failed to create default files: {}", e);
        }

        let listener_fd = self.connection_manager.listener.as_raw_fd();

        let mut total_connections = 0;
        let mut active_connections = 0;

        loop {
            self.accept_new_connections(&mut total_connections, &mut active_connections);
            self.handle_ready_connections(listener_fd, &active_connections);
            self.process_parsed_requests();
            self.cleanup_closed_connections(&mut active_connections);
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn accept_new_connections(
        &self,
        total_connections: &mut usize,
        active_connections: &mut usize,
    ) {
        match self.connection_manager.listener.accept() {
            Ok((stream, addr)) => {
                debug!("New connection from {}", addr);
                if let Err(e) = stream.set_nonblocking(true) {
                    error!("Failed to set non-blocking: {}", e);
                    return;
                }

                if !self.connection_manager.add_connection(stream) {
                    warn!(
                        "Maximum connections reached, rejecting connection from {}",
                        addr
                    );
                } else {
                    *total_connections += 1;
                    *active_connections += 1;
                    info!(
                        "Accepted connection from {} (total: {}, active: {})",
                        addr, total_connections, active_connections
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }

    fn handle_ready_connections(&self, listener_fd: i32, active_connections: &usize) {
        let (mut read_set, mut write_set, mut error_set, max_fd) =
            self.select_handler.prepare_select_sets(listener_fd);

        match self.select_handler.wait_for_events(
            &mut read_set,
            &mut write_set,
            &mut error_set,
            max_fd,
        ) {
            Ok(ready_count) if ready_count > 0 => {
                let (ready_read, ready_write, _) = self.select_handler.get_ready_connections(
                    listener_fd,
                    &read_set,
                    &write_set,
                    &error_set,
                );

                let mut ready_fds = 0;

                for &fd in &ready_read {
                    self.connection_handler.handle_readable_connection(fd);
                    ready_fds += 1;
                }

                for &fd in &ready_write {
                    self.connection_handler.handle_writable_connection(fd);
                    ready_fds += 1;
                }

                self.select_handler
                    .log_ready_connections(ready_fds, active_connections);
            }
            Ok(ready_count) if ready_count == 0 => {}
            Ok(_) => {}
            Err(e) => {
                error!("pselect error: {}", e);
            }
        }
    }

    fn process_parsed_requests(&self) {
        let parse_rx = self.parse_rx.lock().unwrap();
        while let Ok((fd, headers)) = parse_rx.try_recv() {
            self.connection_manager.with_connection(fd, |conn| {
                if conn.stage == ConnectionStage::Parse {
                    conn.headers = headers;
                    conn.headers_sent = 0;
                    conn.stage = ConnectionStage::SendHeaders;
                    debug!("Request parsed and ready to send headers on fd {}", fd);
                }
            });
        }
    }

    fn cleanup_closed_connections(&self, active_connections: &mut usize) {
        let closed_fds = self.connection_manager.get_closed_connections();
        for fd in closed_fds {
            if let Some(conn) = self.connection_manager.remove_connection(fd) {
                *active_connections -= 1;
                if let Ok(addr) = conn.stream.peer_addr() {
                    info!(
                        "Closed connection from {} (active: {})",
                        addr, active_connections
                    );
                } else {
                    info!(
                        "Closed connection on fd {} (active: {})",
                        fd, active_connections
                    );
                }
            }
        }
    }

    fn create_default_files(&self) -> std::io::Result<()> {
        use crate::static_files::{css_content, html_content};
        use std::fs;

        let index_path = self.config.document_root.join("index.html");
        let css_path = self.config.document_root.join("style.css");

        if !self.config.document_root.exists() {
            fs::create_dir_all(&self.config.document_root)?;
        }

        fs::write(index_path, html_content::get_html())?;
        fs::write(css_path, css_content::get_css())?;

        info!(
            "Created default chess-themed page in {:?}",
            self.config.document_root
        );
        Ok(())
    }
}

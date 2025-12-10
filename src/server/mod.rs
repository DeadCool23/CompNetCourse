pub mod config;
pub mod connection_manager;
pub mod http_status;
pub mod request_handler;

use libc::{FD_SET, FD_ZERO, fd_set, pselect, timespec};
use log::{debug, error, info, warn};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use threadpool::ThreadPool;

use crate::server::config::ServerConfig;
use crate::server::connection_manager::ConnectionManager;

pub struct HttpServer {
    config: ServerConfig,
    connection_manager: Arc<ConnectionManager>,
    thread_pool: ThreadPool,
}

impl HttpServer {
    pub fn new(config: &ServerConfig) -> std::io::Result<Self> {
        let addr = format!("{}:{}", config.host, config.port);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(true)?;

        info!("Server started on {}", addr);

        let connection_manager = Arc::new(ConnectionManager::new(listener));
        let thread_pool = ThreadPool::new(config.threads);

        Ok(Self {
            config: config.clone(),
            connection_manager,
            thread_pool,
        })
    }

    pub fn run(&self) {
        info!("Server running with {} threads", self.config.threads);

        if let Err(e) = self.create_default_files() {
            error!("Failed to create default files: {}", e);
        }

        loop {
            self.accept_new_connections();
            self.handle_ready_connections();
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn accept_new_connections(&self) {
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
                    info!(
                        "Accepted connection from {} (total: {})",
                        addr,
                        self.connection_manager.get_connections_fds().len()
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }

    fn handle_ready_connections(&self) {
        let fds = self.connection_manager.get_connections_fds();
        if fds.is_empty() {
            return;
        }

        let mut readfds: fd_set = unsafe { std::mem::zeroed() };
        unsafe { FD_ZERO(&mut readfds) };

        let mut nfds: i32 = 0;
        for &fd in &fds {
            unsafe { FD_SET(fd, &mut readfds) };
            if fd > nfds {
                nfds = fd;
            }
        }

        let timeout = timespec {
            tv_sec: self.config.select_timeout as _,
            tv_nsec: 0,
        };

        let ready_count = unsafe {
            pselect(
                nfds + 1,
                &mut readfds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &timeout,
                std::ptr::null_mut(),
            )
        };

        if ready_count > 0 {
            for fd in fds {
                if unsafe { connection_manager::fd_isset(fd, &mut readfds) } {
                    let connection_manager = Arc::clone(&self.connection_manager);
                    let document_root = self.config.document_root.clone();
                    let max_file_size = self.config.max_file_size;

                    self.thread_pool.execute(move || {
                        if let Some(stream) = connection_manager.get_stream(fd) {
                            request_handler::handle_client(stream, &document_root, max_file_size);
                        }
                    });
                }
            }
        } else if ready_count < 0 {
            error!("pselect error: {}", std::io::Error::last_os_error());
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

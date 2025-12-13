pub mod config;
pub mod connection;
pub mod connection_manager;
mod handlers;
pub mod http_status;

use libc::{fd_set, FD_SET, FD_ISSET, FD_ZERO, pselect, timespec};
use log::{debug, error, info, warn};
use std::net::TcpListener;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use threadpool::ThreadPool;

use config::ServerConfig;
use connection_manager::ConnectionManager;
use handlers::{handle_readable_in_pool, handle_writable_in_pool};

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

        let connection_manager = Arc::new(ConnectionManager::with_config(listener, config));
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

        let listener_fd = self.connection_manager.listener.as_raw_fd();

        let mut total_connections = 0;
        let mut active_connections = 0;

        loop {
            self.accept_new_connections(&mut total_connections, &mut active_connections);
            self.handle_ready_connections(listener_fd, &active_connections);
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
        let (read_fds, write_fds) = self.connection_manager.get_connections_for_select();

        if read_fds.is_empty() && write_fds.is_empty() {
            return;
        }

        let mut read_set: fd_set = unsafe { std::mem::zeroed() };
        let mut write_set: fd_set = unsafe { std::mem::zeroed() };
        let mut error_set: fd_set = unsafe { std::mem::zeroed() };

        unsafe { FD_ZERO(&mut read_set) };
        unsafe { FD_ZERO(&mut write_set) };
        unsafe { FD_ZERO(&mut error_set) };

        unsafe {FD_SET(listener_fd, &mut read_set) };
        let mut max_fd = listener_fd;

        for &fd in &read_fds {
            unsafe { FD_SET(fd, &mut read_set) };
            unsafe { FD_SET(fd, &mut error_set) };
            if fd > max_fd {
                max_fd = fd;
            }
        }

        for &fd in &write_fds {
            unsafe { libc::FD_SET(fd, &mut write_set) };
            unsafe { libc::FD_SET(fd, &mut error_set) };
            if fd > max_fd {
                max_fd = fd;
            }
        }

        let timeout = timespec {
            tv_sec: 0,
            tv_nsec: 10_000_000,
        };

        let ready_count = unsafe {
            pselect(
                max_fd + 1,
                &mut read_set,
                &mut write_set,
                &mut error_set,
                &timeout,
                std::ptr::null_mut(),
            )
        };

        if ready_count > 0 {
            let mut ready_fds = 0;

            for &fd in &read_fds {
                if unsafe { FD_ISSET(fd, &mut read_set) } {
                    let connection_manager = Arc::clone(&self.connection_manager);
                    let doc_root = self.config.document_root.clone();
                    let max_file_size = self.config.max_file_size;

                    self.thread_pool.execute(move || {
                        handle_readable_in_pool(
                            fd,
                            connection_manager,
                            doc_root,
                            max_file_size,
                        );
                    });
                    ready_fds += 1;
                }
            }

            for &fd in &write_fds {
                if unsafe { FD_ISSET(fd, &mut write_set) } {
                    let connection_manager = Arc::clone(&self.connection_manager);

                    self.thread_pool.execute(move || {
                        handle_writable_in_pool(fd, connection_manager);
                    });
                    ready_fds += 1;
                }
            }

            if ready_fds > 0 {
                info!(
                    "pselect found {} ready connections (total: {}, active: {})",
                    ready_fds,
                    self.connection_manager.get_connections_count(),
                    active_connections
                );
            }
        } else if ready_count < 0 {
            error!("pselect error: {}", std::io::Error::last_os_error());
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

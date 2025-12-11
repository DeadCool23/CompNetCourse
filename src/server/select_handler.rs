use libc::{FD_ISSET, FD_SET, FD_ZERO, fd_set, pselect, timespec};
use log::{error, info};
use std::sync::Arc;

use super::connection_manager::ConnectionManager;

pub struct SelectHandler {
    connection_manager: Arc<ConnectionManager>,
    select_timeout: u64,
}

impl SelectHandler {
    pub fn new(connection_manager: Arc<ConnectionManager>, select_timeout: u64) -> Self {
        Self {
            connection_manager,
            select_timeout,
        }
    }

    pub fn prepare_select_sets(&self, listener_fd: i32) -> (fd_set, fd_set, fd_set, i32) {
        let (read_fds, write_fds) = self.connection_manager.get_connections_for_select();

        let mut read_set: fd_set = unsafe { std::mem::zeroed() };
        let mut write_set: fd_set = unsafe { std::mem::zeroed() };
        let mut error_set: fd_set = unsafe { std::mem::zeroed() };

        unsafe { FD_ZERO(&mut read_set) };
        unsafe { FD_ZERO(&mut write_set) };
        unsafe { FD_ZERO(&mut error_set) };

        unsafe { FD_SET(listener_fd, &mut read_set) };
        let mut max_fd = listener_fd;

        for &fd in &read_fds {
            unsafe { FD_SET(fd, &mut read_set) };
            unsafe { FD_SET(fd, &mut error_set) };
            if fd > max_fd {
                max_fd = fd;
            }
        }

        for &fd in &write_fds {
            unsafe { FD_SET(fd, &mut write_set) };
            unsafe { FD_SET(fd, &mut error_set) };
            if fd > max_fd {
                max_fd = fd;
            }
        }

        (read_set, write_set, error_set, max_fd)
    }

    pub fn wait_for_events(
        &self,
        read_set: &mut fd_set,
        write_set: &mut fd_set,
        error_set: &mut fd_set,
        max_fd: i32,
    ) -> Result<i32, std::io::Error> {
        let timeout = timespec {
            tv_sec: self.select_timeout as _,
            tv_nsec: 0,
        };

        let ready_count = unsafe {
            pselect(
                max_fd + 1,
                read_set,
                write_set,
                error_set,
                &timeout,
                std::ptr::null_mut(),
            )
        };

        if ready_count < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(ready_count)
    }

    pub fn get_ready_connections(
        &self,
        listener_fd: i32,
        read_set: &fd_set,
        write_set: &fd_set,
        error_set: &fd_set,
    ) -> (Vec<i32>, Vec<i32>, bool) {
        let mut ready_read = Vec::new();
        let mut ready_write = Vec::new();
        let mut listener_ready = false;

        if unsafe { FD_ISSET(listener_fd, read_set) } {
            listener_ready = true;
        }

        let (all_read_fds, all_write_fds) = self.connection_manager.get_connections_for_select();

        for &fd in &all_read_fds {
            if unsafe { FD_ISSET(fd, read_set) } {
                ready_read.push(fd);
            }
        }

        for &fd in &all_write_fds {
            if unsafe { FD_ISSET(fd, write_set) } {
                ready_write.push(fd);
            }
        }

        for &fd in &all_read_fds {
            if unsafe { FD_ISSET(fd, error_set) } {
                error!("Error on fd {}", fd);
            }
        }

        (ready_read, ready_write, listener_ready)
    }

    pub fn log_ready_connections(&self, ready_fds: usize, active_connections: &usize) {
        if ready_fds > 0 {
            info!(
                "pselect found {} ready connections (total: {}, active: {})",
                ready_fds,
                self.connection_manager.get_connections_count(),
                active_connections
            );
        }
    }
}

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::sync::{Arc, Mutex};

pub struct PtySession {
    pub master: Box<dyn MasterPty + Send>,
    pub child: Arc<Mutex<Option<Box<dyn Child + Send + Sync>>>>,
}

impl PtySession {
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), Box<dyn std::error::Error>> {
        self.master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
        Ok(())
    }

    pub fn try_clone_reader(
        &self,
    ) -> Result<Box<dyn std::io::Read + Send>, Box<dyn std::error::Error>> {
        Ok(self.master.try_clone_reader()?)
    }

    pub fn take_writer(
        &self,
    ) -> Result<Box<dyn std::io::Write + Send>, Box<dyn std::error::Error>> {
        Ok(self.master.take_writer()?)
    }

    pub fn try_wait_child(&self) -> Option<i32> {
        let mut guard = self.child.lock().ok()?;
        if let Some(ref mut child) = *guard {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.exit_code() as i32;
                    drop(guard);
                    self.shutdown();
                    Some(code)
                }
                Ok(None) => None,
                Err(_) => {
                    drop(guard);
                    self.shutdown();
                    Some(-1)
                }
            }
        } else {
            Some(0)
        }
    }

    pub fn shutdown(&self) {
        let mut guard = self.child.lock().ok();
        if let Some(ref mut guard) = guard {
            if let Some(ref mut child) = **guard {
                let _ = child.wait();
            }
            **guard = None;
        }
    }

    pub fn is_alive(&self) -> bool {
        let guard = self.child.lock().ok();
        if let Some(ref guard) = guard {
            guard.is_some()
        } else {
            false
        }
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn create_pty_session(
    shell_path: &str,
    shell_args: &[String],
    cols: u16,
    rows: u16,
) -> Result<PtySession, Box<dyn std::error::Error>> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(shell_path);
    cmd.args(shell_args);
    let child = pair.slave.spawn_command(cmd)?;

    Ok(PtySession {
        master: pair.master,
        child: Arc::new(Mutex::new(Some(child))),
    })
}

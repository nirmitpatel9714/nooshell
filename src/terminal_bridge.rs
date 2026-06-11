use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(windows)]
mod win {
    use std::sync::atomic::AtomicBool;

    unsafe extern "system" {
        fn CreateFileA(
            lpFileName: *const u8,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: isize,
        ) -> isize;
        fn ReadFile(
            hFile: isize,
            lpBuffer: *mut std::ffi::c_void,
            nNumberOfBytesToRead: u32,
            lpNumberOfBytesRead: *mut u32,
            lpOverlapped: *const std::ffi::c_void,
        ) -> i32;
        pub fn CancelIoEx(hFile: isize, lpOverlapped: *const std::ffi::c_void) -> i32;
        pub fn CloseHandle(hObject: isize) -> i32;
    }

    const GENERIC_READ: u32 = 0x8000_0000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;
    const FILE_ATTRIBUTE_NORMAL: u32 = 128;

    pub fn open_conin() -> isize {
        unsafe {
            CreateFileA(
                "CONIN$\0".as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                0,
            )
        }
    }

    pub fn read_conin_blocking(handle: isize, buf: &mut [u8], _running: &AtomicBool) -> Option<usize> {
        unsafe {
            let mut bytes_read: u32 = 0;
            let ret = ReadFile(
                handle,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                buf.len() as u32,
                &mut bytes_read,
                std::ptr::null(),
            );
            if ret == 0 || bytes_read == 0 {
                None
            } else {
                Some(bytes_read as usize)
            }
        }
    }
}

pub struct TerminalBridge {
    running: Arc<AtomicBool>,
    pty_handle: Option<std::thread::JoinHandle<()>>,
    stdin_handle: Option<std::thread::JoinHandle<()>>,
    #[cfg(windows)]
    conin_handle: isize,
}

impl TerminalBridge {
    pub fn start(
        pty_reader: Box<dyn Read + Send>,
        mut pty_writer: Box<dyn Write + Send>,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));

        let r1 = running.clone();
        let pty_handle = std::thread::spawn(move || {
            let _result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut reader = pty_reader;
                    let mut buf = [0u8; 65536];
                    let mut stdout = std::io::stdout();
                    loop {
                        if !r1.load(Ordering::Relaxed) {
                            break;
                        }
                        match reader.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                if stdout.write_all(&buf[..n]).is_err() {
                                    break;
                                }
                                stdout.flush().ok();
                            }
                            Err(ref e)
                                if e.kind() == std::io::ErrorKind::Interrupted =>
                            {
                                continue;
                            }
                            Err(_) => break,
                        }
                    }
                }));
            r1.store(false, Ordering::Relaxed);
        });

        #[cfg(windows)]
        let conin_handle = win::open_conin();

        #[cfg(unix)]
        let conin_handle: isize = -1;

        let r2 = running.clone();
        let stdin_handle = {
            #[cfg(windows)]
            let conin = conin_handle;
            std::thread::spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #[cfg(windows)]
                        {
                            let mut buf = [0u8; 4096];
                            loop {
                                if !r2.load(Ordering::Relaxed) {
                                    break;
                                }
                                if conin != -1 {
                                    if let Some(n) = win::read_conin_blocking(conin, &mut buf, &r2) {
                                        if pty_writer.write_all(&buf[..n]).is_err() {
                                            break;
                                        }
                                        pty_writer.flush().ok();
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                        #[cfg(unix)]
                        {
                            let mut buf = [0u8; 4096];
                            use std::os::unix::io::AsRawFd;
                            loop {
                                if !r2.load(Ordering::Relaxed) {
                                    break;
                                }
                                let fd = std::io::stdin().as_raw_fd();
                                let mut fds = [libc::pollfd {
                                    fd,
                                    events: libc::POLLIN,
                                    revents: 0,
                                }];
                                let ready = unsafe { libc::poll(fds.as_mut_ptr(), 1, 50) > 0 };
                                if ready {
                                    match std::io::stdin().read(&mut buf) {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            if pty_writer.write_all(&buf[..n]).is_err() {
                                                break;
                                            }
                                            pty_writer.flush().ok();
                                        }
                                        Err(ref e)
                                            if e.kind() == std::io::ErrorKind::Interrupted =>
                                        {
                                            continue;
                                        }
                                        Err(_) => break,
                                    }
                                }
                            }
                        }
                    }));
                if result.is_err() {
                    r2.store(false, Ordering::Relaxed);
                }
            })
        };

        TerminalBridge {
            running,
            pty_handle: Some(pty_handle),
            stdin_handle: Some(stdin_handle),
            #[cfg(windows)]
            conin_handle,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn signal_stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for TerminalBridge {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        // Cancel pending I/O on CONIN$ so the stdin thread can wake up
        #[cfg(windows)]
        if self.conin_handle != -1 {
            unsafe { win::CancelIoEx(self.conin_handle, std::ptr::null()); }
        }
        // Join threads first (they should exit after I/O cancellation)
        if let Some(handle) = self.pty_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stdin_handle.take() {
            let _ = handle.join();
        }
        // Close CONIN$ handle only after thread has exited
        #[cfg(windows)]
        if self.conin_handle != -1 {
            unsafe { win::CloseHandle(self.conin_handle); }
        }
    }
}

pub fn enter_passthrough_mode() -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::enable_raw_mode()?;
    Ok(())
}

pub fn exit_passthrough_mode() -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

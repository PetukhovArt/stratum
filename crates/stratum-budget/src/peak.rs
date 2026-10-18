use std::io;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Run {
    pub wall: Duration,
    pub peak_bytes: u64,
    pub code: i32,
}

/// Unix `ru_maxrss` includes waited-for descendants; Windows
/// `PeakWorkingSetSize` counts the child process alone, so peaks differ across platforms.
pub fn run_draining_stdout(cmd: &mut Command) -> io::Result<Run> {
    let start = Instant::now();
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    if let Some(mut out) = child.stdout.take()
        && let Err(e) = io::copy(&mut out, &mut io::sink())
    {
        let _ = child.kill();
        let _ = child.wait();
        return Err(e);
    }
    let (code, peak_bytes) = wait(&mut child)?;
    Ok(Run {
        wall: start.elapsed(),
        peak_bytes,
        code,
    })
}

#[cfg(windows)]
fn wait(child: &mut Child) -> io::Result<(i32, u64)> {
    use std::os::windows::io::AsRawHandle;

    let status = child.wait()?;
    let peak = peak_working_set(child.as_raw_handle())?;
    Ok((status.code().unwrap_or(-1), peak))
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn peak_working_set(handle: std::os::windows::io::RawHandle) -> io::Result<u64> {
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };

    let size = u32::try_from(size_of::<PROCESS_MEMORY_COUNTERS>()).map_err(io::Error::other)?;
    // SAFETY: Win32 FFI contract. The struct is plain integers (all-zero is valid), `cb` is its
    // size, and the handle is the exited child's, which `Child` keeps open.
    let counters = unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = size;
        if GetProcessMemoryInfo(handle, &raw mut counters, size) == 0 {
            return Err(io::Error::last_os_error());
        }
        counters
    };
    u64::try_from(counters.PeakWorkingSetSize).map_err(io::Error::other)
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn wait(child: &mut Child) -> io::Result<(i32, u64)> {
    let pid = libc::pid_t::try_from(child.id()).map_err(io::Error::other)?;
    let mut status = 0;
    // SAFETY: POSIX FFI contract. `rusage` is plain integers (all-zero is valid) and `wait4`
    // writes only through the two out-pointers; reaping the child here is tolerated by `Child`.
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        while libc::wait4(pid, &raw mut status, 0, &raw mut usage) < 0 {
            let err = io::Error::last_os_error();
            if err.kind() != io::ErrorKind::Interrupted {
                return Err(err);
            }
        }
        usage
    };
    let code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        -1
    };
    let max_rss = u64::try_from(usage.ru_maxrss).map_err(io::Error::other)?;
    let bytes = if cfg!(target_os = "macos") {
        max_rss
    } else {
        max_rss * 1024
    };
    Ok((code, bytes))
}

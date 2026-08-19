use core::mem::ManuallyDrop;
use std::io;

use crate::roc_platform_abi::*;
use crate::{os_string_from_native, roc_host, roc_u8_list_from_slice, NativeOsStr};

type CmdExitResult = HostCmdExecExitCodeResult;
type CmdExitResultPayload = HostCmdExecExitCodeResultPayload;
type CmdExitResultTag = HostCmdExecExitCodeResultTag;
type CmdOutputResult = HostCmdExecOutputResult;
type CmdOutputResultPayload = HostCmdExecOutputResultPayload;
type CmdOutputResultTag = HostCmdExecOutputResultTag;
type CmdOutputError = FailedToGetExitCodeOrNonZeroExitCode;
type CmdOutputErrorPayload = FailedToGetExitCodeOrNonZeroExitCodePayload;
type CmdOutputErrorTag = FailedToGetExitCodeOrNonZeroExitCodeTag;
type CmdOutputFailure = HostCmdExecOutputErrNonZeroExitCode;
type CmdOutputSuccess = HostCmdExecOutputOk;
type Cmd = HostCmdExecExitCodeArgs;

fn cmd_io_err_other(message: &str, roc_host: &RocHost) -> HostIOErr {
    HostIOErr {
        payload: HostIOErrPayload {
            other: ManuallyDrop::new(RocStr::from_str(message, roc_host)),
        },
        tag: HostIOErrTag::Other,
    }
}

fn cmd_io_err_from_io(error: &io::Error, roc_host: &RocHost) -> HostIOErr {
    match error.kind() {
        io::ErrorKind::AlreadyExists => HostIOErr {
            payload: HostIOErrPayload { already_exists: [] },
            tag: HostIOErrTag::AlreadyExists,
        },
        io::ErrorKind::BrokenPipe => HostIOErr {
            payload: HostIOErrPayload { broken_pipe: [] },
            tag: HostIOErrTag::BrokenPipe,
        },
        io::ErrorKind::Interrupted => HostIOErr {
            payload: HostIOErrPayload { interrupted: [] },
            tag: HostIOErrTag::Interrupted,
        },
        io::ErrorKind::IsADirectory => HostIOErr {
            payload: HostIOErrPayload { is_adirectory: [] },
            tag: HostIOErrTag::IsADirectory,
        },
        io::ErrorKind::NotFound => HostIOErr {
            payload: HostIOErrPayload { not_found: [] },
            tag: HostIOErrTag::NotFound,
        },
        io::ErrorKind::NotADirectory => HostIOErr {
            payload: HostIOErrPayload { not_adirectory: [] },
            tag: HostIOErrTag::NotADirectory,
        },
        io::ErrorKind::OutOfMemory => HostIOErr {
            payload: HostIOErrPayload { out_of_memory: [] },
            tag: HostIOErrTag::OutOfMemory,
        },
        io::ErrorKind::PermissionDenied => HostIOErr {
            payload: HostIOErrPayload {
                permission_denied: [],
            },
            tag: HostIOErrTag::PermissionDenied,
        },
        io::ErrorKind::Unsupported => HostIOErr {
            payload: HostIOErrPayload { unsupported: [] },
            tag: HostIOErrTag::Unsupported,
        },
        _ => cmd_io_err_other(&error.to_string(), roc_host),
    }
}

fn cmd_output_io_err_other(message: &str, roc_host: &RocHost) -> IOErr {
    IOErr {
        payload: IOErrPayload {
            other: ManuallyDrop::new(RocStr::from_str(message, roc_host)),
        },
        tag: IOErrTag::Other,
    }
}

fn cmd_output_io_err_from_io(error: &io::Error, roc_host: &RocHost) -> IOErr {
    match error.kind() {
        io::ErrorKind::AlreadyExists => IOErr {
            payload: IOErrPayload { already_exists: [] },
            tag: IOErrTag::AlreadyExists,
        },
        io::ErrorKind::BrokenPipe => IOErr {
            payload: IOErrPayload { broken_pipe: [] },
            tag: IOErrTag::BrokenPipe,
        },
        io::ErrorKind::Interrupted => IOErr {
            payload: IOErrPayload { interrupted: [] },
            tag: IOErrTag::Interrupted,
        },
        io::ErrorKind::IsADirectory => IOErr {
            payload: IOErrPayload { is_adirectory: [] },
            tag: IOErrTag::IsADirectory,
        },
        io::ErrorKind::NotFound => IOErr {
            payload: IOErrPayload { not_found: [] },
            tag: IOErrTag::NotFound,
        },
        io::ErrorKind::NotADirectory => IOErr {
            payload: IOErrPayload { not_adirectory: [] },
            tag: IOErrTag::NotADirectory,
        },
        io::ErrorKind::OutOfMemory => IOErr {
            payload: IOErrPayload { out_of_memory: [] },
            tag: IOErrTag::OutOfMemory,
        },
        io::ErrorKind::PermissionDenied => IOErr {
            payload: IOErrPayload {
                permission_denied: [],
            },
            tag: IOErrTag::PermissionDenied,
        },
        io::ErrorKind::Unsupported => IOErr {
            payload: IOErrPayload { unsupported: [] },
            tag: IOErrTag::Unsupported,
        },
        _ => cmd_output_io_err_other(&error.to_string(), roc_host),
    }
}

fn take_arg_list(
    list: &RocList<NativeOsStr>,
    roc_host: &RocHost,
) -> io::Result<Vec<std::ffi::OsString>> {
    let mut values = Vec::with_capacity(list.len());
    let mut first_error = None;

    for item in list.as_slice() {
        match os_string_from_native(*item, roc_host) {
            Ok(value) => values.push(value),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    unsafe { list.decref(roc_host) };

    match first_error {
        Some(error) => Err(error),
        None => Ok(values),
    }
}

fn cmd_to_std(cmd: &Cmd, roc_host: &RocHost) -> io::Result<std::process::Command> {
    let program = os_string_from_native(cmd.program, roc_host);
    let args = take_arg_list(&cmd.args, roc_host);
    let envs = take_arg_list(&cmd.envs, roc_host);

    let mut std_cmd = std::process::Command::new(program?);

    for arg in args? {
        std_cmd.arg(arg);
    }

    if cmd.clear_envs {
        std_cmd.env_clear();
    }

    let envs = envs?;
    debug_assert!(envs.len() % 2 == 0, "envs must come as key value pairs");
    for chunk in envs.chunks(2) {
        if let [key, value] = chunk {
            std_cmd.env(key, value);
        }
    }

    Ok(std_cmd)
}

fn try_cmd_exit_ok(value: i32) -> CmdExitResult {
    CmdExitResult {
        payload: CmdExitResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: CmdExitResultTag::Ok,
    }
}

fn try_cmd_exit_err(error: IOErr) -> CmdExitResult {
    CmdExitResult {
        payload: CmdExitResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: CmdExitResultTag::Err,
    }
}

fn try_cmd_output_ok(value: CmdOutputSuccess) -> CmdOutputResult {
    CmdOutputResult {
        payload: CmdOutputResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: CmdOutputResultTag::Ok,
    }
}

fn try_cmd_output_err(error: CmdOutputError) -> CmdOutputResult {
    CmdOutputResult {
        payload: CmdOutputResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: CmdOutputResultTag::Err,
    }
}

fn cmd_output_nonzero_error(value: CmdOutputFailure) -> CmdOutputError {
    CmdOutputError {
        payload: CmdOutputErrorPayload {
            non_zero_exit_code: ManuallyDrop::new(value),
        },
        tag: CmdOutputErrorTag::NonZeroExitCode,
    }
}

fn cmd_output_failed_to_get_exit_code(error: IOErr) -> CmdOutputError {
    CmdOutputError {
        payload: CmdOutputErrorPayload {
            failed_to_get_exit_code: ManuallyDrop::new(error),
        },
        tag: CmdOutputErrorTag::FailedToGetExitCode,
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_host_exec_exit_code(cmd: Cmd) -> CmdExitResult {
    let roc_host = roc_host();
    let mut std_cmd = match cmd_to_std(&cmd, roc_host) {
        Ok(cmd) => cmd,
        Err(error) => return try_cmd_exit_err(cmd_output_io_err_from_io(&error, roc_host)),
    };

    match std_cmd.status() {
        Ok(status) => match status.code() {
            Some(code) => try_cmd_exit_ok(code),
            // Signal death is an error for this API. The spawned child API
            // reports it as exit code -1 instead, see child_exit_from.
            None => try_cmd_exit_err(cmd_output_io_err_other("Process was killed by signal", roc_host)),
        },
        Err(error) => try_cmd_exit_err(cmd_output_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_host_exec_output(cmd: Cmd) -> CmdOutputResult {
    let roc_host = roc_host();
    let mut std_cmd = match cmd_to_std(&cmd, roc_host) {
        Ok(cmd) => cmd,
        Err(error) => {
            return try_cmd_output_err(cmd_output_failed_to_get_exit_code(
                cmd_output_io_err_from_io(&error, roc_host),
            ))
        }
    };

    match std_cmd.output() {
        Ok(output) => {
            let stdout_bytes = roc_u8_list_from_slice(&output.stdout, roc_host);
            let stderr_bytes = roc_u8_list_from_slice(&output.stderr, roc_host);

            match output.status.code() {
                Some(0) => try_cmd_output_ok(CmdOutputSuccess {
                    stderr_bytes,
                    stdout_bytes,
                }),
                Some(exit_code) => try_cmd_output_err(cmd_output_nonzero_error(CmdOutputFailure {
                    stderr_bytes,
                    stdout_bytes,
                    exit_code,
                })),
                None => {
                    unsafe {
                        stdout_bytes.decref(roc_host);
                        stderr_bytes.decref(roc_host);
                    }
                    try_cmd_output_err(cmd_output_failed_to_get_exit_code(cmd_output_io_err_other(
                        "Process was killed by signal",
                        roc_host,
                    )))
                }
            }
        }
        Err(error) => try_cmd_output_err(cmd_output_failed_to_get_exit_code(
            cmd_output_io_err_from_io(&error, roc_host),
        )),
    }
}

// ============================================================================
// Spawned child processes with piped stdio (Cmd.spawn! / Cmd.spawn_leashed!)
// ============================================================================
//
// Children live in a global table keyed by `u64` handles; the Roc side holds
// only the handle (`Cmd.Child`). stdout/stderr are drained into in-memory
// buffers by background threads started at spawn time, so a chatty child never
// deadlocks on a full OS pipe buffer. `read_stdout`/`read_stderr` serve
// exactly-N-byte reads from those buffers (blocking until enough bytes or
// EOF), which is what length-prefixed protocols (e.g. the Playwright driver)
// need.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Read as _;
use std::io::Write as _;
use std::process::{ChildStdin, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex, MutexGuard};
use std::thread;

#[cfg(not(unix))]
use command_group::{CommandGroup, GroupChild};

/// The Cmd record as generated for `Host.cmd_spawn!` (same layout as
/// `HostCmdExecExitCodeArgs`, but the glue names each hosted fn's argument
/// struct independently).
type SpawnCmd = AnonStruct32ddec9aa3de7110;
type CmdUnitResult = HostCmdChildCloseStdinResult;
type CmdUnitResultPayload = HostCmdChildCloseStdinResultPayload;
type CmdUnitResultTag = HostCmdChildCloseStdinResultTag;
type CmdBytesResult = HostCmdChildReadStderrResult;
type CmdBytesResultPayload = HostCmdChildReadStderrResultPayload;
type CmdBytesResultTag = HostCmdChildReadStderrResultTag;
type ChildExit = AnonStruct3f89ee1e14924626;
// `cmd_child_wait!` and `cmd_child_kill_wait!` return the same Roc type, so the
// glue generator emits one struct and aliases the other name onto it.
type ChildExitResult = HostCmdChildWaitResult;
type ChildExitResultPayload = HostCmdChildWaitResultPayload;
type ChildExitResultTag = HostCmdChildWaitResultTag;

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A child's stdout or stderr, as something we can duplicate into a plain
/// `File`. Owning the pipe as a `File` is what lets the reader thread and a
/// caller share it under one lock.
#[cfg(unix)]
trait PipeSource: std::os::fd::AsFd {}
#[cfg(unix)]
impl<T: std::os::fd::AsFd> PipeSource for T {}
#[cfg(windows)]
trait PipeSource: std::os::windows::io::AsHandle {}
#[cfg(windows)]
impl<T: std::os::windows::io::AsHandle> PipeSource for T {}

/// Take our own read end for the pipe. On Unix it is switched to non-blocking,
/// so a read can report "nothing there" instead of waiting.
#[cfg(unix)]
fn own_read_end<R: PipeSource>(pipe: &R) -> io::Result<File> {
    use std::os::fd::AsRawFd;
    let owned = pipe.as_fd().try_clone_to_owned()?;
    let flags = unsafe { libc::fcntl(owned.as_raw_fd(), libc::F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    if unsafe { libc::fcntl(owned.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(File::from(owned))
}

#[cfg(windows)]
fn own_read_end<R: PipeSource>(pipe: &R) -> io::Result<File> {
    use std::os::windows::io::AsHandle;
    Ok(File::from(pipe.as_handle().try_clone_to_owned()?))
}

/// What a read that refuses to wait found in the pipe.
enum PipeRead {
    Data(usize),
    /// Nothing there right now, though the pipe is still open.
    Empty,
    /// Every writer is gone.
    Closed,
}

#[cfg(unix)]
fn read_without_waiting(pipe: &File, buf: &mut [u8]) -> io::Result<PipeRead> {
    loop {
        return match (&*pipe).read(buf) {
            Ok(0) => Ok(PipeRead::Closed),
            Ok(n) => Ok(PipeRead::Data(n)),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(PipeRead::Empty),
            // Retry on signal interruption rather than reporting an empty pipe,
            // which a caller collecting a dead child's output would believe.
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => Err(e),
        };
    }
}

// `PeekNamedPipe` reports what an anonymous pipe is holding without consuming
// it, which is how a Windows read can tell "nothing there" from "wait here".
// kernel32 is already among the libraries the platform links (see `targets` in
// platform/main.roc).
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn PeekNamedPipe(
        handle: *mut core::ffi::c_void,
        buffer: *mut core::ffi::c_void,
        buffer_size: u32,
        bytes_read: *mut u32,
        total_bytes_available: *mut u32,
        bytes_left_this_message: *mut u32,
    ) -> i32;
}

#[cfg(windows)]
fn read_without_waiting(pipe: &File, buf: &mut [u8]) -> io::Result<PipeRead> {
    use std::os::windows::io::AsRawHandle;

    let mut available: u32 = 0;
    let peeked = unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            core::ptr::null_mut(),
            0,
            core::ptr::null_mut(),
            &mut available,
            core::ptr::null_mut(),
        )
    };
    if peeked == 0 {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::BrokenPipe {
            return Ok(PipeRead::Closed);
        }
        return Err(error);
    }
    if available == 0 {
        return Ok(PipeRead::Empty);
    }

    // Asking for no more than the peek promised keeps this read from waiting.
    let want = (available as usize).min(buf.len());
    loop {
        return match (&*pipe).read(&mut buf[..want]) {
            Ok(0) => Ok(PipeRead::Closed),
            Ok(n) => Ok(PipeRead::Data(n)),
            Err(ref e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(PipeRead::Closed),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => Err(e),
        };
    }
}

/// Wait until the pipe has something to say, without taking anything out of it.
#[cfg(unix)]
fn wait_for_pipe(pipe: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let mut poll_fd = libc::pollfd {
        fd: pipe.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        if unsafe { libc::poll(&mut poll_fd, 1, -1) } >= 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

/// Windows cannot wait on an anonymous pipe for readability, so look again in a
/// moment instead. Only an idle stream pays for this: while output is flowing
/// the reader never gets here.
#[cfg(windows)]
fn wait_for_pipe(_pipe: &File) -> io::Result<()> {
    thread::sleep(std::time::Duration::from_millis(1));
    Ok(())
}

/// Move what the pipe is holding into the buffer, and say whether it has
/// closed. The caller must hold the stream lock: taking bytes out of the pipe
/// only under the lock is what stops a chunk from living in a reader's hands,
/// where neither the buffer nor the pipe accounts for it.
///
/// `budget` caps how many reads one turn may make. The reader thread uses it to
/// give the lock back now and then, so that a child writing without pause
/// cannot lock out the calls that read what it wrote. Passing None collects
/// until the pipe runs dry, which is what a caller taking final delivery wants.
fn collect_available(
    pipe: &File,
    state: &mut StreamState,
    cv: &Condvar,
    budget: Option<usize>,
) -> io::Result<Collected> {
    let mut chunk = [0u8; 16 * 1024];
    let mut reads = 0;
    loop {
        if budget.is_some_and(|budget| reads >= budget) {
            return Ok(Collected::More);
        }
        reads += 1;
        match read_without_waiting(pipe, &mut chunk)? {
            PipeRead::Data(n) => {
                state.data.extend(&chunk[..n]);
                cv.notify_all();
            }
            PipeRead::Empty => return Ok(Collected::Drained),
            PipeRead::Closed => return Ok(Collected::Closed),
        }
    }
}

/// How a turn of [collect_available] ended.
enum Collected {
    /// The pipe ran dry.
    Drained,
    /// The turn's budget ran out with bytes still waiting.
    More,
    /// Every writer is gone.
    Closed,
}

/// Reads one turn of the reader thread may make before handing the lock back.
/// 128KB at a time keeps a chatty child from locking out the calls that read
/// what it wrote, without making the common small-output case pay for extra
/// round trips.
const READ_BUDGET: usize = 8;

/// Background reader that drains a child's stdout or stderr pipe into an
/// in-memory buffer, starting at spawn time.
///
/// Without it nothing reads the pipe until the child exits. A child that
/// writes more than the OS pipe buffer (around 64KB) then blocks on its next
/// write and never exits, so poll and wait hang until a timeout kills it.
/// Draining from spawn lets the child keep writing.
///
/// The buffer is unbounded. A child that writes without limit grows it
/// without limit, so we trade the deadlock for memory use. That is fine for
/// the normal case of KB to MB of output.
type SharedStream = Arc<(Mutex<StreamState>, Condvar)>;

struct StreamReader {
    shared: SharedStream,
    pipe: Option<Arc<File>>,
    handle: Option<thread::JoinHandle<()>>,
}

struct StreamState {
    data: VecDeque<u8>,
    eof: bool,
    /// Set once nobody will look at `data` again, so the reader thread can
    /// retire instead of buffering output for a reader that has gone away.
    stop: bool,
    err: Option<io::Error>,
}

impl StreamReader {
    /// Start a thread draining `pipe` into a buffer. A None pipe gives back an
    /// already-closed reader with no bytes and immediate EOF.
    fn spawn<R: PipeSource>(pipe: Option<R>) -> io::Result<Self> {
        let pipe = match &pipe {
            Some(pipe) => Some(Arc::new(own_read_end(pipe)?)),
            None => None,
        };

        let shared = Arc::new((
            Mutex::new(StreamState {
                data: VecDeque::new(),
                eof: pipe.is_none(),
                stop: false,
                err: None,
            }),
            Condvar::new(),
        ));

        let handle = pipe.as_ref().map(|pipe| {
            let pipe = Arc::clone(pipe);
            let shared = Arc::clone(&shared);
            thread::spawn(move || {
                let (lock, cv) = &*shared;
                loop {
                    let collected = {
                        let mut state = lock_or_recover(lock);
                        if state.stop {
                            return;
                        }
                        match collect_available(&pipe, &mut state, cv, Some(READ_BUDGET)) {
                            Ok(collected) => collected,
                            Err(e) => {
                                state.err = Some(e);
                                state.eof = true;
                                cv.notify_all();
                                return;
                            }
                        }
                    };

                    match collected {
                        Collected::Closed => {
                            let mut state = lock_or_recover(lock);
                            state.eof = true;
                            cv.notify_all();
                            return;
                        }
                        // More is waiting, so go straight back for it. The lock
                        // was released on the way past, which was the point.
                        Collected::More => {}
                        // Wait for more outside the lock, so a caller can look
                        // at the pipe while this thread has nothing in hand.
                        Collected::Drained => {
                            if let Err(e) = wait_for_pipe(&pipe) {
                                let mut state = lock_or_recover(lock);
                                state.err = Some(e);
                                state.eof = true;
                                cv.notify_all();
                                return;
                            }
                        }
                    }
                }
            })
        });

        Ok(StreamReader {
            shared,
            pipe,
            handle,
        })
    }

    /// Block until exactly `num_bytes` are buffered, then take them from the
    /// front. If the stream reaches EOF with fewer bytes available it returns
    /// UnexpectedEof.
    ///
    /// Takes the shared state rather than `&self` so that callers can hand it a
    /// clone and block on the read without holding the process table lock.
    fn read_exact_n(shared: &SharedStream, num_bytes: u64) -> io::Result<Vec<u8>> {
        let n = num_bytes as usize;
        let (lock, cv) = &**shared;
        let mut state = lock_or_recover(lock);
        loop {
            if state.data.len() >= n {
                return Ok(state.data.drain(..n).collect());
            }
            if let Some(err) = state.err.take() {
                return Err(err);
            }
            if state.eof {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            state = cv.wait(state).unwrap_or_else(|p| p.into_inner());
        }
    }

    /// Take everything the child left behind without waiting for the pipe to
    /// close. Only complete once the process is known to be gone: its writes
    /// finished before it did, so everything it produced is either buffered
    /// here already or sitting in the OS pipe, and this collects both.
    ///
    /// Holding the lock is what makes that airtight. The reader thread only
    /// touches the pipe under the same lock, so while we have it no bytes are
    /// in flight anywhere, and the pipe is ours to empty.
    ///
    /// This is what [StreamReader::drain_remaining] cannot do. Waiting for the
    /// pipe to close means waiting for *every* process holding it, so a
    /// grandchild that outlives the child can keep a caller waiting forever.
    /// Output such a grandchild writes after this call is not collected, which
    /// is the price of never waiting on it. The one survivor that can still
    /// draw this out is one writing hard enough that the pipe never runs dry,
    /// since that is the condition this stops on.
    fn drain_pending(&mut self) -> io::Result<Vec<u8>> {
        let (lock, cv) = &*self.shared;
        let mut state = lock_or_recover(lock);

        if let Some(pipe) = &self.pipe {
            if !state.eof && !state.stop {
                match collect_available(pipe, &mut state, cv, None) {
                    Ok(Collected::Closed) => state.eof = true,
                    Ok(_) => {}
                    Err(e) => {
                        state.stop = true;
                        return Err(e);
                    }
                }
            }
        }

        // The reader thread outlives us whenever something else still holds the
        // pipe. Retire it rather than let it buffer output nobody will read.
        state.stop = true;
        if let Some(err) = state.err.take() {
            return Err(err);
        }
        Ok(state.data.drain(..).collect())
    }

    /// Wait for the stream to close and return everything still buffered.
    /// Called by wait once the child has exited or is about to. We join the
    /// drain thread before taking the lock, not while holding it, so we can't
    /// deadlock against the thread that needs the lock to push its last bytes.
    fn drain_remaining(&mut self) -> io::Result<Vec<u8>> {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let (lock, _cv) = &*self.shared;
        let mut state = lock_or_recover(lock);
        if let Some(err) = state.err.take() {
            return Err(err);
        }
        Ok(state.data.drain(..).collect())
    }
}

/// A spawned child, either standalone or leashed. On Unix a leashed child
/// lives in the process group its Watchdog leads; on Windows it lives in a
/// Job Object (via command-group) that dies with this process.
enum ChildHandle {
    Plain(std::process::Child),
    #[cfg(unix)]
    Leashed {
        child: std::process::Child,
        /// The group the Watchdog founded, or the child's own pid when the
        /// watchdog fork failed and the child leads its own group instead.
        pgid: libc::pid_t,
    },
    #[cfg(not(unix))]
    Leashed(GroupChild),
}

impl ChildHandle {
    fn kill(&mut self) -> io::Result<()> {
        match self {
            ChildHandle::Plain(c) => c.kill(),
            // The whole group goes down at once, watchdog included. std's
            // Child::kill would take down only the direct child and leave
            // grandchildren running.
            #[cfg(unix)]
            ChildHandle::Leashed { pgid, .. } => {
                if unsafe { libc::kill(-*pgid, libc::SIGKILL) } == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            }
            #[cfg(not(unix))]
            ChildHandle::Leashed(c) => c.kill(),
        }
    }
    fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        match self {
            ChildHandle::Plain(c) => c.wait(),
            #[cfg(unix)]
            ChildHandle::Leashed { child, .. } => child.wait(),
            #[cfg(not(unix))]
            ChildHandle::Leashed(c) => c.wait(),
        }
    }
    fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        match self {
            ChildHandle::Plain(c) => c.try_wait(),
            #[cfg(unix)]
            ChildHandle::Leashed { child, .. } => child.try_wait(),
            #[cfg(not(unix))]
            ChildHandle::Leashed(c) => c.try_wait(),
        }
    }
}

/// A spawned process. stdout and stderr are drained into in-memory buffers by
/// background threads started at spawn time (see StreamReader). stdin stays a
/// raw handle that we write to on demand.
struct Process {
    child: ChildHandle,
    leashed: bool,
    /// Behind its own lock so a write that blocks on a full pipe stalls only
    /// this child, never the process table.
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: StreamReader,
    stderr: StreamReader,
    /// Present for leashed children on Unix, None when its fork failed.
    /// Never read, held so its Drop runs when the entry leaves the table.
    #[cfg(unix)]
    _watchdog: Option<Watchdog>,
}

/// Takes a leashed child's whole process group down when this process dies,
/// however it dies. A forked helper leads the group and blocks reading a pipe
/// whose write end lives only in this process, so any death of this process
/// closes the pipe, the read returns EOF, and the helper SIGKILLs the group.
/// This is what cleans up after Ctrl+C, crashes, and `kill -9`, none of which
/// run the exit sweep in `rust_main`. Linux additionally has PDEATHSIG, but
/// that covers only the direct child, while the group kill here also reaps
/// grandchildren.
///
/// The helper forks before the child spawns and founds the group itself, so
/// the group provably exists by the time the child joins it, and the pgid is
/// pinned against reuse for exactly as long as the leash is held. The helper
/// execs nothing (no /bin/sh required on the system, so this works in
/// scratch containers) and blocks every signal, so nothing short of SIGKILL
/// can strip the leash.
///
/// The liveness pipe is deliberately separate from the child's stdin. Closing
/// a child's stdin is the normal way to signal end of input and must not read
/// as a death sentence.
#[cfg(unix)]
struct Watchdog {
    /// The helper's pid, which is also the pgid of the group it leads.
    pgid: libc::pid_t,
    /// Closing this is what wakes the watchdog, see Drop.
    liveness: Option<io::PipeWriter>,
}

#[cfg(unix)]
impl Watchdog {
    /// A watchdog that fails to fork is reported as None rather than as a
    /// spawn failure: the child still spawns, leading its own group, which is
    /// the behavior all leashed children had before watchdogs existed.
    fn fork() -> Option<Watchdog> {
        use std::os::fd::AsRawFd;
        let (reader, writer) = io::pipe().ok()?;
        // The helper's fd sweep bound has to be taken on this side of the
        // fork: getrlimit is not on the async-signal-safe list.
        let fd_limit = fd_sweep_limit();
        match unsafe { libc::fork() } {
            -1 => None,
            0 => unsafe { watchdog_main(reader.as_raw_fd(), fd_limit) },
            helper => {
                let watchdog = Watchdog {
                    pgid: helper,
                    liveness: Some(writer),
                };
                // Found the group from this side as well: the helper's own
                // setpgid may not have run yet, and the child joins the group
                // as soon as this returns. setpgid on a forked child that has
                // not exec'd cannot fail with EACCES, which is what makes the
                // group's existence deterministic rather than a race.
                if unsafe { libc::setpgid(helper, helper) } == 0 {
                    Some(watchdog)
                } else {
                    // Dropping the watchdog wakes and reaps the helper.
                    None
                }
            }
        }
    }
}

/// The body of the forked watchdog helper.
///
/// SAFETY FENCE: this runs in the child of a fork from a multithreaded
/// process, where POSIX allows only async-signal-safe operations until
/// _exit. Everything in here must stay raw libc: no allocation, no locks, no
/// panics, no Rust std I/O. (`io::Error::last_os_error().raw_os_error()` is
/// fine, it only reads errno.)
#[cfg(unix)]
unsafe fn watchdog_main(liveness_fd: libc::c_int, fd_limit: libc::c_int) -> ! {
    // Block everything blockable: handlers inherited from the host can never
    // run in here, and stray signals to the group cannot quietly kill the
    // watchdog and strip the leash.
    let mut all: libc::sigset_t = std::mem::zeroed();
    libc::sigfillset(&mut all);
    libc::sigprocmask(libc::SIG_BLOCK, &all, std::ptr::null_mut());

    // Found the group. The parent does this too, whoever runs first wins.
    libc::setpgid(0, 0);

    // Keep only the liveness read end. Everything else inherited from the
    // fork gets closed, most importantly the liveness write ends of sibling
    // watchdogs, which would otherwise never reach EOF while this helper
    // lives, and the host's own stdio pipes.
    libc::dup2(liveness_fd, 0);
    close_fds_from(1, fd_limit);

    // Nothing is ever written to the pipe, so the only successful read is
    // EOF, and it means the sole holder of the write end, the host process,
    // is gone. Signals are blocked, but stay robust against EINTR anyway.
    let mut byte = 0u8;
    loop {
        let n = libc::read(0, &mut byte as *mut u8 as *mut libc::c_void, 1);
        if n >= 0 || io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            break;
        }
    }

    // Nuke the group, but only if this helper actually leads it: killing by
    // explicit pgid (never kill(0)) cannot hit the host's own group through
    // some earlier setpgid failure.
    let own_pid = libc::getpid();
    if libc::getpgrp() == own_pid {
        libc::kill(-own_pid, libc::SIGKILL);
    }
    libc::_exit(0)
}

/// Close every fd in [first, limit) inside the forked helper. Only
/// async-signal-safe calls, see the fence on watchdog_main.
#[cfg(unix)]
unsafe fn close_fds_from(first: libc::c_int, limit: libc::c_int) {
    // One syscall on Linux 5.9+. The loop covers older kernels and the
    // Unixes without close_range (macOS).
    #[cfg(target_os = "linux")]
    if libc::syscall(
        libc::SYS_close_range,
        first as libc::c_uint,
        libc::c_uint::MAX,
        0 as libc::c_uint,
    ) == 0
    {
        return;
    }
    close_fds_loop(first, limit);
}

/// The fallback sweep for platforms without close_range. Factored out so the
/// tests can exercise it on Linux, where close_range normally shadows it.
/// Async-signal-safe.
#[cfg(unix)]
unsafe fn close_fds_loop(first: libc::c_int, limit: libc::c_int) {
    for fd in first..limit {
        libc::close(fd);
    }
}

/// Upper bound for the helper's fd sweep: fd numbers are capped by the soft
/// NOFILE limit. Clamped in case the limit is set to unlimited.
#[cfg(unix)]
fn fd_sweep_limit() -> libc::c_int {
    let mut rl = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    let soft = if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut rl) } == 0 {
        rl.rlim_cur
    } else {
        1024
    };
    soft.min(1 << 20) as libc::c_int
}

#[cfg(unix)]
impl Drop for Watchdog {
    fn drop(&mut self) {
        // Dropping the write end wakes the watchdog, which group-kills any
        // stragglers (the child itself is dead or already being killed
        // whenever its Process entry is dropped) and dies with them, being in
        // the same group. The wait is bounded by that and keeps the helper
        // from lingering as a zombie.
        self.liveness.take();
        let mut status: libc::c_int = 0;
        while unsafe { libc::waitpid(self.pgid, &mut status, 0) } == -1
            && io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
        {}
    }
}

static PROCESSES: LazyLock<Mutex<std::collections::HashMap<u64, Process>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static NEXT_PROCESS_ID: AtomicU64 = AtomicU64::new(1);

fn process_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "Process not found")
}

/// The Unix leash: fork the watchdog, found the group, and spawn the child
/// into it. Shared by spawn_impl and the tests, so the tests exercise the
/// real path.
///
/// Three things stop a leashed child from outliving this process: the exit
/// sweep in rust_main on normal exits, the Watchdog on deaths that skip it
/// (Ctrl+C, crashes, kill -9), and on Linux also PDEATHSIG, which needs no
/// second process and fires without the watchdog's EOF latency.
#[cfg(unix)]
fn leash_spawn(
    std_cmd: &mut std::process::Command,
) -> io::Result<(std::process::Child, libc::pid_t, Option<Watchdog>)> {
    use std::os::unix::process::CommandExt;

    #[cfg(target_os = "linux")]
    unsafe {
        std_cmd.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    // The watchdog forks first and founds the group, so the group already
    // exists when the child joins it: no retry loop, and no window in which
    // the child runs without its leash pinned. When the fork failed the
    // child leads its own group, unleashed, which is the pre-watchdog
    // behavior.
    let watchdog = Watchdog::fork();
    std_cmd.process_group(watchdog.as_ref().map_or(0, |w| w.pgid));
    let child = std_cmd.spawn()?;
    let pgid = watchdog
        .as_ref()
        .map_or(child.id() as libc::pid_t, |w| w.pgid);
    Ok((child, pgid, watchdog))
}

/// The Windows leash: a Job Object with kill-on-close, so the kernel takes
/// the whole job down when the last handle to it closes, which any death of
/// this process does (clean exit, abort, TerminateProcess). Kill-on-close
/// must be asked for: `group_spawn()` alone leaves command-group's
/// kill_on_drop at its false default, which builds the job without
/// JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE and reduces the leash to explicit
/// kills only.
#[cfg(not(unix))]
fn leash_spawn(std_cmd: &mut std::process::Command) -> io::Result<GroupChild> {
    std_cmd.group().kill_on_drop(true).spawn()
}

fn spawn_impl(cmd: &Cmd, leashed: bool, roc_host: &RocHost) -> io::Result<u64> {
    let mut std_cmd = cmd_to_std(cmd, roc_host)?;
    std_cmd.stdin(Stdio::piped());
    std_cmd.stdout(Stdio::piped());
    std_cmd.stderr(Stdio::piped());

    #[cfg(unix)]
    let (mut child, stdin, stdout_pipe, stderr_pipe, watchdog) = if leashed {
        let (mut child, pgid, watchdog) = leash_spawn(&mut std_cmd)?;
        let stdin = child.stdin.take();
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        (
            ChildHandle::Leashed { child, pgid },
            stdin,
            stdout_pipe,
            stderr_pipe,
            watchdog,
        )
    } else {
        let mut child = std_cmd.spawn()?;
        let stdin = child.stdin.take();
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        (
            ChildHandle::Plain(child),
            stdin,
            stdout_pipe,
            stderr_pipe,
            None,
        )
    };

    #[cfg(not(unix))]
    let (mut child, stdin, stdout_pipe, stderr_pipe) = if leashed {
        let mut child = leash_spawn(&mut std_cmd)?;
        let stdin = child.inner().stdin.take();
        let stdout_pipe = child.inner().stdout.take();
        let stderr_pipe = child.inner().stderr.take();
        (ChildHandle::Leashed(child), stdin, stdout_pipe, stderr_pipe)
    } else {
        let mut child = std_cmd.spawn()?;
        let stdin = child.stdin.take();
        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();
        (ChildHandle::Plain(child), stdin, stdout_pipe, stderr_pipe)
    };

    // The child is already running here. A reader that fails to start must
    // take it down, or it runs on with no handle anywhere to reach it by.
    let readers = StreamReader::spawn(stdout_pipe)
        .and_then(|stdout| Ok((stdout, StreamReader::spawn(stderr_pipe)?)));
    let (stdout, stderr) = match readers {
        Ok(readers) => readers,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };

    let process = Process {
        child,
        leashed,
        stdin: Arc::new(Mutex::new(stdin)),
        stdout,
        stderr,
        #[cfg(unix)]
        _watchdog: watchdog,
    };

    let process_id = NEXT_PROCESS_ID.fetch_add(1, Ordering::Relaxed);
    lock_or_recover(&PROCESSES).insert(process_id, process);
    Ok(process_id)
}

fn kill_process(process: &mut Process) -> io::Result<()> {
    process.child.kill()?;
    let _ = process.child.wait();
    Ok(())
}

/// Kill every leashed child still in the table. Called from `rust_main` on
/// program exit and from `hosted_cmd_kill_all_leashed`.
pub(crate) fn kill_all_leashed_children() {
    let mut processes = lock_or_recover(&PROCESSES);
    let leashed_ids: Vec<u64> = processes
        .iter()
        .filter(|(_, p)| p.leashed)
        .map(|(id, _)| *id)
        .collect();
    for id in leashed_ids {
        if let Some(mut process) = processes.remove(&id) {
            let _ = kill_process(&mut process);
        }
    }
}

fn cmd_unit_ok() -> CmdUnitResult {
    CmdUnitResult {
        payload: CmdUnitResultPayload { ok: [] },
        tag: CmdUnitResultTag::Ok,
    }
}

fn cmd_unit_err(error: HostIOErr) -> CmdUnitResult {
    CmdUnitResult {
        payload: CmdUnitResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: CmdUnitResultTag::Err,
    }
}

fn cmd_unit_result(result: io::Result<()>, roc_host: &RocHost) -> CmdUnitResult {
    match result {
        Ok(()) => cmd_unit_ok(),
        Err(e) => cmd_unit_err(cmd_io_err_from_io(&e, roc_host)),
    }
}

fn cmd_bytes_result(result: io::Result<Vec<u8>>, roc_host: &RocHost) -> CmdBytesResult {
    match result {
        Ok(bytes) => CmdBytesResult {
            payload: CmdBytesResultPayload {
                ok: ManuallyDrop::new(roc_u8_list_from_slice(&bytes, roc_host)),
            },
            tag: CmdBytesResultTag::Ok,
        },
        Err(e) => CmdBytesResult {
            payload: CmdBytesResultPayload {
                err: ManuallyDrop::new(cmd_output_io_err_from_io(&e, roc_host)),
            },
            tag: CmdBytesResultTag::Err,
        },
    }
}

fn child_exit_from(status: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>, roc_host: &RocHost) -> ChildExit {
    ChildExit {
        stderr_bytes: roc_u8_list_from_slice(&stderr, roc_host),
        stdout_bytes: roc_u8_list_from_slice(&stdout, roc_host),
        // -1 when the process was terminated by a signal.
        exit_code: status.code().unwrap_or(-1),
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_spawn(cmd: SpawnCmd, leashed: bool) -> HostCmdSpawnResult {
    let roc_host = roc_host();
    let cmd = Cmd {
        args: cmd.args,
        envs: cmd.envs,
        program: cmd.program,
        clear_envs: cmd.clear_envs,
    };
    match spawn_impl(&cmd, leashed, roc_host) {
        Ok(id) => HostCmdSpawnResult {
            payload: HostCmdSpawnResultPayload {
                ok: ManuallyDrop::new(id),
            },
            tag: HostCmdSpawnResultTag::Ok,
        },
        Err(e) => HostCmdSpawnResult {
            payload: HostCmdSpawnResultPayload {
                err: ManuallyDrop::new(cmd_output_io_err_from_io(&e, roc_host)),
            },
            tag: HostCmdSpawnResultTag::Err,
        },
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_write_stdin(
    process_id: u64,
    bytes: RocListWith<u8, false>,
) -> CmdUnitResult {
    let roc_host = roc_host();
    // Clone the stdin handle out of the table before writing. A write blocks
    // when the child lets the pipe fill up, and other children must stay
    // usable while it does.
    let result = (|| {
        let stdin = {
            let processes = lock_or_recover(&PROCESSES);
            let process = processes.get(&process_id).ok_or_else(process_not_found)?;
            Arc::clone(&process.stdin)
        };
        let mut stdin = lock_or_recover(&stdin);
        match *stdin {
            Some(ref mut handle) => {
                handle.write_all(bytes.as_slice())?;
                handle.flush()
            }
            None => Err(io::Error::other("Process stdin not available")),
        }
    })();
    unsafe { bytes.decref(roc_host) };
    cmd_unit_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_read_stdout(process_id: u64, num_bytes: u64) -> CmdBytesResult {
    let roc_host = roc_host();
    // Clone the reader's shared handle so we don't hold the table lock while
    // blocking; read_exact_n can wait indefinitely for the child to produce
    // output, and other children must stay usable meanwhile.
    let result = (|| {
        let shared = {
            let processes = lock_or_recover(&PROCESSES);
            let process = processes.get(&process_id).ok_or_else(process_not_found)?;
            Arc::clone(&process.stdout.shared)
        };
        StreamReader::read_exact_n(&shared, num_bytes)
    })();
    cmd_bytes_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_read_stderr(process_id: u64, num_bytes: u64) -> CmdBytesResult {
    let roc_host = roc_host();
    let result = (|| {
        let shared = {
            let processes = lock_or_recover(&PROCESSES);
            let process = processes.get(&process_id).ok_or_else(process_not_found)?;
            Arc::clone(&process.stderr.shared)
        };
        StreamReader::read_exact_n(&shared, num_bytes)
    })();
    cmd_bytes_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_close_stdin(process_id: u64) -> CmdUnitResult {
    let roc_host = roc_host();
    let result = (|| {
        let stdin = {
            let processes = lock_or_recover(&PROCESSES);
            let process = processes.get(&process_id).ok_or_else(process_not_found)?;
            Arc::clone(&process.stdin)
        };
        *lock_or_recover(&stdin) = None;
        Ok(())
    })();
    cmd_unit_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_kill(process_id: u64) -> CmdUnitResult {
    let roc_host = roc_host();
    let result = (|| {
        let mut process = lock_or_recover(&PROCESSES)
            .remove(&process_id)
            .ok_or_else(process_not_found)?;
        kill_process(&mut process)
    })();
    cmd_unit_result(result, roc_host)
}

fn child_exit_result(
    result: io::Result<(std::process::ExitStatus, Vec<u8>, Vec<u8>)>,
    roc_host: &RocHost,
) -> ChildExitResult {
    match result {
        Ok((status, stdout, stderr)) => ChildExitResult {
            payload: ChildExitResultPayload {
                ok: ManuallyDrop::new(child_exit_from(status, stdout, stderr, roc_host)),
            },
            tag: ChildExitResultTag::Ok,
        },
        Err(e) => ChildExitResult {
            payload: ChildExitResultPayload {
                err: ManuallyDrop::new(cmd_output_io_err_from_io(&e, roc_host)),
            },
            tag: ChildExitResultTag::Err,
        },
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_wait(process_id: u64) -> ChildExitResult {
    let roc_host = roc_host();
    let result = (|| {
        let mut process = lock_or_recover(&PROCESSES)
            .remove(&process_id)
            .ok_or_else(process_not_found)?;
        // The drain threads have been emptying the pipes since spawn. Collect
        // what they buffered and wait for EOF, then reap the child. Reap even
        // when a drain fails, or the child lingers as a zombie.
        let stdout = process.stdout.drain_remaining();
        let stderr = process.stderr.drain_remaining();
        let status = process.child.wait()?;
        Ok((status, stdout?, stderr?))
    })();
    child_exit_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_kill_wait(process_id: u64) -> ChildExitResult {
    let roc_host = roc_host();
    let result = (|| {
        let mut process = lock_or_recover(&PROCESSES)
            .remove(&process_id)
            .ok_or_else(process_not_found)?;

        // A child that already exited on its own cannot be killed, and some
        // platforms report that as an error. It is not one here: we still want
        // the status and output below, and the real exit code is better than a
        // synthesised one when a child beats us to the finish by a hair.
        let killed = match process.child.kill() {
            Ok(()) => Ok(()),
            Err(error) => match process.child.try_wait() {
                Ok(Some(_)) => Ok(()),
                Ok(None) => Err(error),
                Err(wait_error) => Err(wait_error),
            },
        };

        if let Err(error) = killed {
            // The child outlived our attempt on it, so put it back where
            // `kill_leashed!` and the exit-time sweep can still find it.
            lock_or_recover(&PROCESSES).insert(process_id, process);
            return Err(error);
        }

        // Kill before collecting, the opposite order from `wait!`. The reader
        // threads only see EOF once every writer has dropped the pipe, so
        // draining a live child would block until it exited on its own, which
        // is exactly what the caller is trying to avoid.
        //
        // Reaping first is what makes the collection below both complete and
        // bounded: the child's writes finished before it died, so its output is
        // already buffered or already in the pipe, and `drain_pending` takes
        // both without waiting on a grandchild that outlived the kill. For a
        // leashed child there is no such grandchild, since the whole tree went
        // down with the group.
        let status = process.child.wait()?;
        let stdout = process.stdout.drain_pending()?;
        let stderr = process.stderr.drain_pending()?;
        Ok((status, stdout, stderr))
    })();
    child_exit_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_poll(process_id: u64) -> HostCmdChildPollResult {
    let roc_host = roc_host();
    let result = (|| {
        let mut processes = lock_or_recover(&PROCESSES);
        let process = processes
            .get_mut(&process_id)
            .ok_or_else(process_not_found)?;
        match process.child.try_wait()? {
            // The child is gone, so collect what it wrote without waiting for
            // the pipe to close. Waiting would break the one promise `poll!`
            // makes, since a surviving grandchild can hold the pipe open long
            // after the child it belonged to has exited.
            Some(status) => {
                let stdout = process.stdout.drain_pending()?;
                let stderr = process.stderr.drain_pending()?;
                processes.remove(&process_id);
                Ok(Some((status, stdout, stderr)))
            }
            None => Ok(None),
        }
    })();
    match result {
        Ok(Some((status, stdout, stderr))) => HostCmdChildPollResult {
            payload: HostCmdChildPollResultPayload {
                ok: ManuallyDrop::new(ExitedOrRunning {
                    payload: ExitedOrRunningPayload {
                        exited: ManuallyDrop::new(child_exit_from(status, stdout, stderr, roc_host)),
                    },
                    tag: ExitedOrRunningTag::Exited,
                }),
            },
            tag: HostCmdChildPollResultTag::Ok,
        },
        Ok(None) => HostCmdChildPollResult {
            payload: HostCmdChildPollResultPayload {
                ok: ManuallyDrop::new(ExitedOrRunning {
                    payload: ExitedOrRunningPayload { running: [] },
                    tag: ExitedOrRunningTag::Running,
                }),
            },
            tag: HostCmdChildPollResultTag::Ok,
        },
        Err(e) => HostCmdChildPollResult {
            payload: HostCmdChildPollResultPayload {
                err: ManuallyDrop::new(cmd_output_io_err_from_io(&e, roc_host)),
            },
            tag: HostCmdChildPollResultTag::Err,
        },
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_kill_all_leashed() -> CmdUnitResult {
    kill_all_leashed_children();
    cmd_unit_ok()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::io::BufRead as _;
    use std::os::unix::process::ExitStatusExt as _;
    use std::time::Duration;

    /// Polls until `pid` no longer exists. Zombies still "exist" for
    /// kill(0), so for another process's orphans this also waits out the
    /// reparent-to-init and reap that follows their parent's death.
    fn wait_until_gone(pid: libc::pid_t) {
        for _ in 0..1000 {
            if unsafe { libc::kill(pid, 0) } == -1 {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("process {pid} still alive after 10s");
    }

    fn sleeper() -> std::process::Command {
        let mut cmd = std::process::Command::new("sleep");
        cmd.arg("600");
        cmd
    }

    #[test]
    fn dropping_the_watchdog_takes_the_group_down() {
        let (mut child, pgid, watchdog) = leash_spawn(&mut sleeper()).unwrap();
        let watchdog = watchdog.expect("watchdog fork failed");
        assert_eq!(pgid, watchdog.pgid);
        drop(watchdog);
        let status = child.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        wait_until_gone(pgid);
    }

    #[test]
    fn the_pgid_stays_pinned_after_the_child_exits() {
        let mut cmd = std::process::Command::new("true");
        let (mut child, pgid, watchdog) = leash_spawn(&mut cmd).unwrap();
        assert!(watchdog.is_some(), "watchdog fork failed");
        child.wait().unwrap();
        // The group must survive its founding child: the helper holds it, so
        // an exit-time kill(-pgid) can never hit a recycled group.
        assert_eq!(
            unsafe { libc::kill(-pgid, 0) },
            0,
            "group vanished with the child"
        );
        drop(watchdog);
        wait_until_gone(pgid);
    }

    /// Two leashes: the second helper forks while the first one's liveness
    /// write end is open in this process, so only the fd sweep keeps it from
    /// holding that write end and stalling the first leash's EOF.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_helper_keeps_only_its_liveness_fd() {
        let (mut child_a, _pgid_a, watchdog_a) = leash_spawn(&mut sleeper()).unwrap();
        let (mut child_b, pgid_b, watchdog_b) = leash_spawn(&mut sleeper()).unwrap();
        assert!(watchdog_b.is_some(), "watchdog fork failed");
        let fd_dir = format!("/proc/{pgid_b}/fd");
        let mut fds: Vec<String> = Vec::new();
        for _ in 0..300 {
            fds = std::fs::read_dir(&fd_dir)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect();
            if fds == ["0"] {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fds, ["0"], "helper still holds inherited fds");
        drop(watchdog_a);
        drop(watchdog_b);
        let _ = child_a.wait();
        let _ = child_b.wait();
    }

    /// Parses the fake host's stdout for the tree it leashed: pgid and child
    /// from its own "leash-host" line, the grandchild pid from the relayed
    /// "leash-grandchild" line.
    fn read_tree_announcement(
        reader: impl std::io::BufRead,
    ) -> (libc::pid_t, libc::pid_t, libc::pid_t) {
        let mut pgid: libc::pid_t = 0;
        let mut child: libc::pid_t = 0;
        let mut grandchild: libc::pid_t = 0;
        for line in reader.lines() {
            let line = line.unwrap();
            if let Some(rest) = line.strip_prefix("leash-host ") {
                let mut parts = rest.split(' ');
                pgid = parts.next().unwrap().parse().unwrap();
                child = parts.next().unwrap().parse().unwrap();
            } else if let Some(rest) = line.strip_prefix("leash-grandchild ") {
                grandchild = rest.trim().parse().unwrap();
            }
            if pgid != 0 && grandchild != 0 {
                break;
            }
        }
        (pgid, child, grandchild)
    }

    /// Re-runs this test binary as a disposable host (the ignored fake_host
    /// test below), which leashes a child-plus-grandchild tree, announces
    /// the pids on stdout, and dies the way `mode` says. The leash must take
    /// the whole tree down for every way the host can die, including the
    /// ones that skip all cleanup.
    fn host_death_takes_the_tree_down(mode: &str) {
        let exe = std::env::current_exe().unwrap();
        let mut host = std::process::Command::new(exe)
            .args(["cmd::tests::fake_host", "--exact", "--ignored", "--nocapture"])
            .env("LEASH_TEST_DEATH_MODE", mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = std::io::BufReader::new(host.stdout.take().unwrap());
        let (pgid, child, grandchild) = read_tree_announcement(stdout);
        assert!(
            pgid > 0 && child > 0 && grandchild > 0,
            "fake host never announced its tree (mode {mode})"
        );
        host.wait().unwrap();
        wait_until_gone(child);
        wait_until_gone(grandchild);
        // The watchdog helper must not linger either.
        wait_until_gone(pgid);
    }

    #[test]
    fn tree_dies_when_the_host_exits_without_cleanup() {
        host_death_takes_the_tree_down("exit");
    }

    #[test]
    fn tree_dies_when_the_host_is_sigkilled() {
        host_death_takes_the_tree_down("sigkill");
    }

    #[test]
    fn tree_dies_when_the_host_aborts() {
        host_death_takes_the_tree_down("abort");
    }

    #[test]
    fn tree_dies_when_the_host_is_sigtermed() {
        host_death_takes_the_tree_down("sigterm");
    }

    /// Runs the close_range-less sweep (the macOS path, shadowed by the
    /// close_range syscall on modern Linux) in a forked child and asserts it
    /// closes exactly the fds at or above the floor. Only async-signal-safe
    /// calls in the child: fcntl and _exit.
    #[test]
    fn the_fallback_fd_sweep_closes_only_fds_at_or_above_the_floor() {
        use std::os::fd::AsRawFd as _;
        let (reader, writer) = io::pipe().unwrap();
        let probe_a = reader.as_raw_fd();
        let probe_b = writer.as_raw_fd();
        assert!(probe_a >= 3 && probe_b >= 3);
        let limit = fd_sweep_limit();
        match unsafe { libc::fork() } {
            -1 => panic!("fork failed"),
            0 => unsafe {
                close_fds_loop(3, limit);
                let mut failures = 0;
                for fd in [0, 1, 2] {
                    if libc::fcntl(fd, libc::F_GETFD) == -1 {
                        failures |= 1; // swept below the floor
                    }
                }
                for fd in [probe_a, probe_b] {
                    if libc::fcntl(fd, libc::F_GETFD) != -1 {
                        failures |= 2; // missed an fd above the floor
                    }
                }
                libc::_exit(failures);
            },
            pid => {
                let mut status: libc::c_int = 0;
                assert_ne!(unsafe { libc::waitpid(pid, &mut status, 0) }, -1);
                assert!(libc::WIFEXITED(status), "sweep child died abnormally");
                assert_eq!(
                    libc::WEXITSTATUS(status),
                    0,
                    "1 = swept below floor, 2 = missed above floor"
                );
            }
        }
    }

    /// True once the helper's sigprocmask has run, so the signal test below
    /// cannot race the helper's startup. Read from /proc on Linux, from
    /// `ps -o blocked=` elsewhere.
    #[cfg(target_os = "linux")]
    fn helper_blocks_sigterm(pid: libc::pid_t) -> bool {
        let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status")) else {
            return false;
        };
        status.lines().any(|line| {
            line.strip_prefix("SigBlk:").is_some_and(|hex| {
                u64::from_str_radix(hex.trim(), 16)
                    .is_ok_and(|mask| mask >> (libc::SIGTERM - 1) & 1 == 1)
            })
        })
    }

    /// macOS exposes no way to read another process's signal mask (`ps -o
    /// blocked=` reports 0 there no matter what the mask holds), so watch for
    /// the fd sweep instead: watchdog_main runs it strictly after
    /// sigprocmask, so the mask is up once fd 0 is the helper's only open
    /// file.
    #[cfg(not(target_os = "linux"))]
    fn helper_blocks_sigterm(pid: libc::pid_t) -> bool {
        const PROC_PIDLISTFDS: libc::c_int = 1;
        const FDINFO_SIZE: usize = 8; // sizeof(struct proc_fdinfo)
        let mut buf = [0u8; 64 * FDINFO_SIZE];
        let bytes = unsafe {
            libc::proc_pidinfo(
                pid,
                PROC_PIDLISTFDS,
                0,
                buf.as_mut_ptr().cast(),
                buf.len() as libc::c_int,
            )
        };
        // Exactly one open fd, and it is fd 0 (the liveness pipe).
        bytes as usize == FDINFO_SIZE
            && i32::from_ne_bytes(buf[..4].try_into().unwrap()) == 0
    }

    /// Only SIGKILL may strip the leash: a catchable signal sent to the
    /// whole group kills the child but must leave the helper (and with it
    /// the backstop and the pgid pin) standing.
    #[test]
    fn a_signal_to_the_group_does_not_strip_the_leash() {
        let (mut child, pgid, watchdog) = leash_spawn(&mut sleeper()).unwrap();
        assert!(watchdog.is_some(), "watchdog fork failed");
        for _ in 0..1000 {
            if helper_blocks_sigterm(pgid) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(helper_blocks_sigterm(pgid), "helper never blocked signals");
        assert_eq!(unsafe { libc::kill(-pgid, libc::SIGTERM) }, 0);
        let status = child.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGTERM));
        assert_eq!(
            unsafe { libc::kill(-pgid, 0) },
            0,
            "the SIGTERM stripped the leash"
        );
        drop(watchdog);
        wait_until_gone(pgid);
    }

    /// The slave side of a PTY master: ptsname_r where it exists, plain
    /// ptsname elsewhere (fine here, the tests hold no other PTYs).
    #[cfg(target_os = "linux")]
    fn pty_slave_path(master: std::os::fd::RawFd) -> String {
        let mut name = [0 as libc::c_char; 128];
        assert_eq!(
            unsafe { libc::ptsname_r(master, name.as_mut_ptr(), name.len()) },
            0
        );
        unsafe { std::ffi::CStr::from_ptr(name.as_ptr()) }
            .to_str()
            .unwrap()
            .to_owned()
    }

    #[cfg(not(target_os = "linux"))]
    fn pty_slave_path(master: std::os::fd::RawFd) -> String {
        let name = unsafe { libc::ptsname(master) };
        assert!(!name.is_null(), "ptsname failed");
        unsafe { std::ffi::CStr::from_ptr(name) }
            .to_str()
            .unwrap()
            .to_owned()
    }

    /// Ctrl+C at a real terminal: SIGINT goes to the foreground process
    /// group only. The fake host becomes a session leader on a fresh PTY, so
    /// the ^C written to the master kills it, while the leash group sits
    /// outside the foreground group, survives the keystroke, and is then
    /// taken down by the watchdog noticing the host's death.
    #[test]
    fn tree_dies_when_the_host_gets_ctrl_c() {
        use std::os::fd::{AsRawFd as _, FromRawFd as _};
        use std::os::unix::process::CommandExt as _;

        let master = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
        assert!(master >= 0, "posix_openpt failed");
        // CLOEXEC keeps the master out of the host and its tree. A leaked
        // master can never be closed by this test, and closing the master is
        // the only way out of the macOS exit-path wedge described below.
        assert_ne!(unsafe { libc::fcntl(master, libc::F_SETFD, libc::FD_CLOEXEC) }, -1);
        let master = unsafe { std::fs::File::from_raw_fd(master) };
        unsafe {
            assert_eq!(libc::grantpt(master.as_raw_fd()), 0);
            assert_eq!(libc::unlockpt(master.as_raw_fd()), 0);
        }
        let slave_path = pty_slave_path(master.as_raw_fd());
        let slave = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&slave_path)
            .unwrap();

        // Make sure the line discipline turns ^C into SIGINT. ECHO must be
        // off: nothing ever reads the master after the ^C goes in, and on
        // macOS an exiting session leader waits in the kernel for the tty's
        // output queue (which the echoed "^C" would sit in) to drain. With
        // echo on, the host wedges unkillable in its exit path, and every
        // process in the session wedges behind it, until the master closes.
        unsafe {
            let mut tio: libc::termios = std::mem::zeroed();
            assert_eq!(libc::tcgetattr(slave.as_raw_fd(), &mut tio), 0);
            tio.c_lflag |= libc::ISIG;
            tio.c_lflag &= !libc::ECHO;
            assert_eq!(libc::tcsetattr(slave.as_raw_fd(), libc::TCSANOW, &tio), 0);
        }

        let exe = std::env::current_exe().unwrap();
        let mut cmd = std::process::Command::new(exe);
        cmd.args(["cmd::tests::fake_host", "--exact", "--ignored", "--nocapture"])
            .env("LEASH_TEST_DEATH_MODE", "sigint")
            .stdin(Stdio::from(slave))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                // Fresh session with the PTY (on fd 0) as the controlling
                // terminal and this process's group in the foreground.
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                if libc::ioctl(0, libc::TIOCSCTTY as _, 0) == -1 {
                    return Err(io::Error::last_os_error());
                }
                if libc::tcsetpgrp(0, libc::getpgrp()) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut host = cmd.spawn().unwrap();
        let reader = std::io::BufReader::new(host.stdout.take().unwrap());
        let (pgid, child, grandchild) = read_tree_announcement(reader);
        assert!(
            pgid > 0 && child > 0 && grandchild > 0,
            "fake host never announced its tree"
        );

        assert_eq!(
            unsafe { libc::write(master.as_raw_fd(), b"\x03".as_ptr().cast(), 1) },
            1
        );
        let status = host.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGINT), "host did not die from ^C");
        wait_until_gone(child);
        wait_until_gone(grandchild);
        wait_until_gone(pgid);
    }

    /// Not a test: the disposable host that host_death_takes_the_tree_down
    /// re-execs. Ignored so normal runs skip it; a bare `--ignored` run
    /// without the env var makes it a no-op.
    #[test]
    #[ignore]
    fn fake_host() {
        let Ok(mode) = std::env::var("LEASH_TEST_DEATH_MODE") else {
            return;
        };
        let mut cmd = std::process::Command::new("sh");
        cmd.args([
            "-c",
            "sleep 600 & echo leash-grandchild $!; exec sleep 600",
        ]);
        cmd.stdout(Stdio::piped());
        let (mut child, pgid, watchdog) = leash_spawn(&mut cmd).unwrap();
        // Relay the grandchild announcement rather than letting the sh write
        // it to the shared stdout: reading it here is what guarantees the
        // grandchild exists before this host dies, otherwise the group kill
        // races the echo and the tree is announced incompletely.
        let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut announcement = String::new();
        reader.read_line(&mut announcement).unwrap();
        print!("{announcement}");
        println!("leash-host {} {}", pgid, child.id());
        std::io::stdout().flush().unwrap();
        // Neither the watchdog nor the child may be dropped: a Drop here
        // would clean the tree up and the death below would prove nothing.
        std::mem::forget(watchdog);
        std::mem::forget(child);
        match mode.as_str() {
            "exit" => std::process::exit(0),
            "sigkill" => {
                unsafe { libc::raise(libc::SIGKILL) };
            }
            "abort" => std::process::abort(),
            "sigterm" => {
                unsafe { libc::raise(libc::SIGTERM) };
            }
            // For the Ctrl+C test: stay alive until the ^C on the
            // controlling terminal delivers SIGINT.
            "sigint" => loop {
                thread::sleep(Duration::from_secs(600));
            },
            other => panic!("unknown death mode {other}"),
        }
        panic!("still alive after death mode {mode}");
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::io::BufRead as _;
    use std::io::Write as _;
    use std::time::Duration;

    /// Polls tasklist until `pid` no longer exists. The padded needle keeps
    /// pid 123 from matching pid 1234's row.
    fn wait_until_gone(pid: u32) {
        let filter = format!("PID eq {pid}");
        let needle = format!(" {pid} ");
        for _ in 0..100 {
            let out = std::process::Command::new("tasklist")
                .args(["/NH", "/FI", &filter])
                .output()
                .unwrap();
            if !String::from_utf8_lossy(&out.stdout).contains(&needle) {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("process {pid} still alive after 10s");
    }

    /// The Windows spin on the unix death matrix: re-runs this binary as a
    /// disposable host that leashes a powershell child plus a ping
    /// grandchild through the real Job Object path (leash_spawn), announces
    /// the pids, and dies per `mode`. Kill-on-close must take the whole job
    /// down for every way the host can die.
    fn host_death_takes_the_tree_down(mode: &str) {
        let exe = std::env::current_exe().unwrap();
        let mut host = std::process::Command::new(exe)
            .args([
                "cmd::windows_tests::fake_host",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env("LEASH_TEST_DEATH_MODE", mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let reader = std::io::BufReader::new(host.stdout.take().unwrap());
        let mut child: u32 = 0;
        let mut grandchild: u32 = 0;
        for line in reader.lines() {
            let line = line.unwrap();
            if let Some(rest) = line.strip_prefix("leash-host ") {
                child = rest.trim().parse().unwrap();
            } else if let Some(rest) = line.strip_prefix("leash-grandchild ") {
                grandchild = rest.trim().parse().unwrap();
            }
            if child != 0 && grandchild != 0 {
                break;
            }
        }
        assert!(
            child > 0 && grandchild > 0,
            "fake host never announced its tree (mode {mode})"
        );
        if mode == "hang" {
            // An outside TerminateProcess: the kill -9 analog.
            let killed = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &host.id().to_string()])
                .status()
                .unwrap();
            assert!(killed.success(), "taskkill failed");
        }
        host.wait().unwrap();
        wait_until_gone(child);
        wait_until_gone(grandchild);
    }

    #[test]
    fn tree_dies_when_the_host_exits_without_cleanup() {
        host_death_takes_the_tree_down("exit");
    }

    #[test]
    fn tree_dies_when_the_host_aborts() {
        host_death_takes_the_tree_down("abort");
    }

    #[test]
    fn tree_dies_when_the_host_is_terminated() {
        host_death_takes_the_tree_down("hang");
    }

    /// Not a test: the disposable host, see the unix twin for the pattern.
    #[test]
    #[ignore]
    fn fake_host() {
        let Ok(mode) = std::env::var("LEASH_TEST_DEATH_MODE") else {
            return;
        };
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-Command",
            "$p = Start-Process ping -ArgumentList '-n','600','127.0.0.1' -PassThru -WindowStyle Hidden; \
             Write-Host ('leash-grandchild ' + $p.Id); Start-Sleep -Seconds 600",
        ]);
        cmd.stdout(Stdio::piped());
        let mut child = leash_spawn(&mut cmd).unwrap();
        // Relaying the announcement guarantees the grandchild exists before
        // this host dies, same as the unix fake host.
        let mut reader = std::io::BufReader::new(child.inner().stdout.take().unwrap());
        let mut announcement = String::new();
        reader.read_line(&mut announcement).unwrap();
        print!("{announcement}");
        println!("leash-host {}", child.id());
        std::io::stdout().flush().unwrap();
        // Not dropped: a Drop here would close the job handle and clean the
        // tree up before the death below gets to prove anything.
        std::mem::forget(child);
        match mode.as_str() {
            "exit" => std::process::exit(0),
            "abort" => std::process::abort(),
            // For the terminated test: stay alive until the outside
            // taskkill lands.
            "hang" => loop {
                thread::sleep(Duration::from_secs(600));
            },
            other => panic!("unknown death mode {other}"),
        }
    }
}

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
// Spawned child processes with piped stdio (Cmd.spawn! / Cmd.spawn_grouped!)
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

/// A spawned child, either standalone or in a process group (spawn_grouped).
enum ChildHandle {
    Plain(std::process::Child),
    Grouped(GroupChild),
}

impl ChildHandle {
    fn kill(&mut self) -> io::Result<()> {
        match self {
            ChildHandle::Plain(c) => c.kill(),
            ChildHandle::Grouped(c) => c.kill(),
        }
    }
    fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        match self {
            ChildHandle::Plain(c) => c.wait(),
            ChildHandle::Grouped(c) => c.wait(),
        }
    }
    fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        match self {
            ChildHandle::Plain(c) => c.try_wait(),
            ChildHandle::Grouped(c) => c.try_wait(),
        }
    }
}

/// A spawned process. stdout and stderr are drained into in-memory buffers by
/// background threads started at spawn time (see StreamReader). stdin stays a
/// raw handle that we write to on demand.
struct Process {
    child: ChildHandle,
    grouped: bool,
    /// Behind its own lock so a write that blocks on a full pipe stalls only
    /// this child, never the process table.
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: StreamReader,
    stderr: StreamReader,
}

static PROCESSES: LazyLock<Mutex<std::collections::HashMap<u64, Process>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static NEXT_PROCESS_ID: AtomicU64 = AtomicU64::new(1);

fn process_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "Process not found")
}

fn spawn_impl(cmd: &Cmd, grouped: bool, roc_host: &RocHost) -> io::Result<u64> {
    let mut std_cmd = cmd_to_std(cmd, roc_host)?;
    std_cmd.stdin(Stdio::piped());
    std_cmd.stdout(Stdio::piped());
    std_cmd.stderr(Stdio::piped());

    // On Linux the child dies with the parent (reliable even when the parent
    // is SIGKILLed); elsewhere the managed group still catches normal exits.
    #[cfg(target_os = "linux")]
    if grouped {
        use std::os::unix::process::CommandExt;
        unsafe {
            std_cmd.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let (mut child, stdin, stdout_pipe, stderr_pipe) = if grouped {
        let mut child = std_cmd.group_spawn()?;
        let stdin = child.inner().stdin.take();
        let stdout_pipe = child.inner().stdout.take();
        let stderr_pipe = child.inner().stderr.take();
        (ChildHandle::Grouped(child), stdin, stdout_pipe, stderr_pipe)
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
        grouped,
        stdin: Arc::new(Mutex::new(stdin)),
        stdout,
        stderr,
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

/// Kill every grouped child still in the table. Called from `rust_main` on
/// program exit and from `hosted_cmd_kill_all_grouped`.
pub(crate) fn kill_all_grouped_children() {
    let mut processes = lock_or_recover(&PROCESSES);
    let grouped_ids: Vec<u64> = processes
        .iter()
        .filter(|(_, p)| p.grouped)
        .map(|(id, _)| *id)
        .collect();
    for id in grouped_ids {
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
pub extern "C" fn hosted_cmd_spawn(cmd: SpawnCmd, grouped: bool) -> HostCmdSpawnResult {
    let roc_host = roc_host();
    let cmd = Cmd {
        args: cmd.args,
        envs: cmd.envs,
        program: cmd.program,
        clear_envs: cmd.clear_envs,
    };
    match spawn_impl(&cmd, grouped, roc_host) {
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
            // `kill_grouped!` and the exit-time sweep can still find it.
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
        // grouped child there is no such grandchild, since the whole tree went
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
pub extern "C" fn hosted_cmd_kill_all_grouped() -> CmdUnitResult {
    kill_all_grouped_children();
    cmd_unit_ok()
}

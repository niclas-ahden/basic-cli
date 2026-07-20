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
use std::io::Write as _;
use std::process::{ChildStdin, Stdio};
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

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
struct StreamReader {
    shared: Arc<(Mutex<StreamState>, Condvar)>,
    handle: Option<thread::JoinHandle<()>>,
}

struct StreamState {
    data: VecDeque<u8>,
    eof: bool,
    err: Option<io::Error>,
}

impl StreamReader {
    /// Start a thread draining `pipe` into a buffer. A None pipe gives back an
    /// already-closed reader with no bytes and immediate EOF.
    fn spawn<R: io::Read + Send + 'static>(pipe: Option<R>) -> Self {
        let shared = Arc::new((
            Mutex::new(StreamState {
                data: VecDeque::new(),
                eof: pipe.is_none(),
                err: None,
            }),
            Condvar::new(),
        ));

        let handle = pipe.map(|mut pipe| {
            let shared = Arc::clone(&shared);
            thread::spawn(move || {
                let (lock, cv) = &*shared;
                let mut chunk = [0u8; 16 * 1024];
                loop {
                    match pipe.read(&mut chunk) {
                        Ok(0) => {
                            lock_or_recover(lock).eof = true;
                            cv.notify_all();
                            break;
                        }
                        Ok(n) => {
                            lock_or_recover(lock).data.extend(&chunk[..n]);
                            cv.notify_all();
                        }
                        // Retry on signal interruption rather than treating it as EOF.
                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
                        Err(e) => {
                            let mut state = lock_or_recover(lock);
                            state.err = Some(e);
                            state.eof = true;
                            cv.notify_all();
                            break;
                        }
                    }
                }
            })
        });

        StreamReader { shared, handle }
    }

    /// Block until exactly `num_bytes` are buffered, then take them from the
    /// front. If the stream reaches EOF with fewer bytes available it returns
    /// UnexpectedEof.
    fn read_exact_n(&self, num_bytes: u64) -> io::Result<Vec<u8>> {
        let n = num_bytes as usize;
        let (lock, cv) = &*self.shared;
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

    /// Wait for the stream to close and return everything still buffered.
    /// Called by wait and poll once the child has exited or is about to. We
    /// join the drain thread before taking the lock, not while holding it, so
    /// we can't deadlock against the thread that needs the lock to push its
    /// last bytes.
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
    stdin: Option<ChildStdin>,
    stdout: StreamReader,
    stderr: StreamReader,
}

static PROCESSES: LazyLock<Mutex<std::collections::HashMap<u64, Process>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static NEXT_PROCESS_ID: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(1));

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

    let process = if grouped {
        let mut child = std_cmd.group_spawn()?;
        Process {
            stdin: child.inner().stdin.take(),
            stdout: StreamReader::spawn(child.inner().stdout.take()),
            stderr: StreamReader::spawn(child.inner().stderr.take()),
            child: ChildHandle::Grouped(child),
            grouped,
        }
    } else {
        let mut child = std_cmd.spawn()?;
        Process {
            stdin: child.stdin.take(),
            stdout: StreamReader::spawn(child.stdout.take()),
            stderr: StreamReader::spawn(child.stderr.take()),
            child: ChildHandle::Plain(child),
            grouped,
        }
    };

    let process_id = {
        let mut next_id = lock_or_recover(&NEXT_PROCESS_ID);
        let id = *next_id;
        *next_id += 1;
        id
    };
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
    let result = (|| {
        let mut processes = lock_or_recover(&PROCESSES);
        let process = processes
            .get_mut(&process_id)
            .ok_or_else(process_not_found)?;
        match process.stdin {
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
        StreamReader {
            shared,
            handle: None,
        }
        .read_exact_n(num_bytes)
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
        StreamReader {
            shared,
            handle: None,
        }
        .read_exact_n(num_bytes)
    })();
    cmd_bytes_result(result, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_cmd_child_close_stdin(process_id: u64) -> CmdUnitResult {
    let roc_host = roc_host();
    let result = (|| {
        let mut processes = lock_or_recover(&PROCESSES);
        let process = processes
            .get_mut(&process_id)
            .ok_or_else(process_not_found)?;
        process.stdin = None;
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

#[no_mangle]
pub extern "C" fn hosted_cmd_child_wait(process_id: u64) -> HostCmdChildWaitResult {
    let roc_host = roc_host();
    let result = (|| {
        let mut process = lock_or_recover(&PROCESSES)
            .remove(&process_id)
            .ok_or_else(process_not_found)?;
        // The drain threads have been emptying the pipes since spawn. Collect
        // what they buffered and wait for EOF, then reap the child.
        let stdout = process.stdout.drain_remaining()?;
        let stderr = process.stderr.drain_remaining()?;
        let status = process.child.wait()?;
        Ok((status, stdout, stderr))
    })();
    match result {
        Ok((status, stdout, stderr)) => HostCmdChildWaitResult {
            payload: HostCmdChildWaitResultPayload {
                ok: ManuallyDrop::new(child_exit_from(status, stdout, stderr, roc_host)),
            },
            tag: HostCmdChildWaitResultTag::Ok,
        },
        Err(e) => HostCmdChildWaitResult {
            payload: HostCmdChildWaitResultPayload {
                err: ManuallyDrop::new(cmd_output_io_err_from_io(&e, roc_host)),
            },
            tag: HostCmdChildWaitResultTag::Err,
        },
    }
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
            Some(status) => {
                let stdout = process.stdout.drain_remaining()?;
                let stderr = process.stderr.drain_remaining()?;
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

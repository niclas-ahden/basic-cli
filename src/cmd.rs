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

fn try_cmd_exit_err(error: HostIOErr) -> CmdExitResult {
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
        Err(error) => return try_cmd_exit_err(cmd_io_err_from_io(&error, roc_host)),
    };

    match std_cmd.status() {
        Ok(status) => match status.code() {
            Some(code) => try_cmd_exit_ok(code),
            None => try_cmd_exit_err(cmd_io_err_other("Process was killed by signal", roc_host)),
        },
        Err(error) => try_cmd_exit_err(cmd_io_err_from_io(&error, roc_host)),
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

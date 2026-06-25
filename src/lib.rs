//! Roc platform host implementation for Roc's direct-symbol host ABI.

#![allow(improper_ctypes_definitions)]

use core::mem::ManuallyDrop;
use std::ffi::{c_char, c_void, CStr};
use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

mod roc_platform_abi;

use crate::roc_platform_abi::*;

// RustGlue generates numbered names for anonymous Roc records and result types.
// Keep those generated names localized here so host code can use API-level names.
type CmdExitResult = TryType0;
type CmdExitResultPayload = TryType0Payload;
type CmdExitResultTag = TryType0Tag;
type CmdIOErr = IOErrType1;
type CmdIOErrPayload = IOErrType1Payload;
type CmdIOErrTag = IOErrType1Tag;
type CmdOutputResult = TryType7;
type CmdOutputResultPayload = TryType7Payload;
type CmdOutputResultTag = TryType7Tag;
type CmdOutputFailureResult = TryType8;
type CmdOutputFailureResultPayload = TryType8Payload;
type CmdOutputFailureResultTag = TryType8Tag;
type CmdOutputFailure = AnonStruct9;
type CmdOutputSuccess = AnonStruct12;

type DirUnitResult = TryType13;
type DirUnitResultPayload = TryType13Payload;
type DirUnitResultTag = TryType13Tag;
type DirIOErr = IOErrType15;
type DirIOErrPayload = IOErrType15Payload;
type DirIOErrTag = IOErrType15Tag;
type DirListResult = TryType18;
type DirListResultPayload = TryType18Payload;
type DirListResultTag = TryType18Tag;

type EnvVarResult = TryType21;
type EnvVarResultPayload = TryType21Payload;
type EnvVarResultTag = TryType21Tag;
type EnvCwdResult = TryType24;
type EnvCwdResultPayload = TryType24Payload;
type EnvCwdResultTag = TryType24Tag;
type EnvExePathResult = TryType27;
type EnvExePathResultPayload = TryType27Payload;
type EnvExePathResultTag = TryType27Tag;

type FileBytesResult = TryType29;
type FileBytesResultPayload = TryType29Payload;
type FileBytesResultTag = TryType29Tag;
type FileIOErr = IOErrType31;
type FileIOErrPayload = IOErrType31Payload;
type FileIOErrTag = IOErrType31Tag;
type FileWriteBytesResult = TryType35;
type FileWriteBytesResultPayload = TryType35Payload;
type FileWriteBytesResultTag = TryType35Tag;
type FileStrResult = TryType38;
type FileStrResultPayload = TryType38Payload;
type FileStrResultTag = TryType38Tag;
type FileWriteUtf8Result = TryType40;
type FileWriteUtf8ResultPayload = TryType40Payload;
type FileWriteUtf8ResultTag = TryType40Tag;
type FileDeleteResult = TryType42;
type FileDeleteResultPayload = TryType42Payload;
type FileDeleteResultTag = TryType42Tag;
type FileSizeResult = TryType44;
type FileSizeResultPayload = TryType44Payload;
type FileSizeResultTag = TryType44Tag;
type FileBoolResult = TryType47;
type FileBoolResultPayload = TryType47Payload;
type FileBoolResultTag = TryType47Tag;
type FileTimeResult = TryType50;
type FileTimeResultPayload = TryType50Payload;
type FileTimeResultTag = TryType50Tag;

type LocaleGetResult = TryType53;
type LocaleGetResultPayload = TryType53Payload;
type LocaleGetResultTag = TryType53Tag;

type PathTypeResult = TryType57;
type PathTypeResultPayload = TryType57Payload;
type PathTypeResultTag = TryType57Tag;
type PathIOErr = IOErrType58;
type PathIOErrPayload = IOErrType58Payload;
type PathIOErrTag = IOErrType58Tag;
type PathInfo = AnonStruct60;

type RandomU64Result = TryType64;
type RandomU64ResultPayload = TryType64Payload;
type RandomU64ResultTag = TryType64Tag;
type RandomIOErr = IOErrType66;
type RandomIOErrPayload = IOErrType66Payload;
type RandomIOErrTag = IOErrType66Tag;
type RandomU32Result = TryType70;
type RandomU32ResultPayload = TryType70Payload;
type RandomU32ResultTag = TryType70Tag;

type StderrUnitResult = TryType75;
type StderrUnitResultPayload = TryType75Payload;
type StderrUnitResultTag = TryType75Tag;
type StderrBytesResult = TryType80;
type StderrBytesResultPayload = TryType80Payload;
type StderrBytesResultTag = TryType80Tag;
type StderrIOErr = IOErrType77;
type StderrIOErrPayload = IOErrType77Payload;
type StderrIOErrTag = IOErrType77Tag;

type StdinLineResult = TryType84;
type StdinLineResultPayload = TryType84Payload;
type StdinLineResultTag = TryType84Tag;
type StdinLineReadErr = EndOfFileOrStdinErrType85;
type StdinLineReadErrPayload = EndOfFileOrStdinErrType85Payload;
type StdinLineReadErrTag = EndOfFileOrStdinErrType85Tag;
type StdinIOErr = IOErrType86;
type StdinIOErrPayload = IOErrType86Payload;
type StdinIOErrTag = IOErrType86Tag;
type StdinBytesResult = TryType89;
type StdinBytesResultPayload = TryType89Payload;
type StdinBytesResultTag = TryType89Tag;
type StdinBytesReadErr = EndOfFileOrStdinErrType90;
type StdinBytesReadErrPayload = EndOfFileOrStdinErrType90Payload;
type StdinBytesReadErrTag = EndOfFileOrStdinErrType90Tag;
type StdinReadToEndResult = TryType93;
type StdinReadToEndResultPayload = TryType93Payload;
type StdinReadToEndResultTag = TryType93Tag;

type StdoutUnitResult = TryType95;
type StdoutUnitResultPayload = TryType95Payload;
type StdoutUnitResultTag = TryType95Tag;
type StdoutBytesResult = TryType100;
type StdoutBytesResultPayload = TryType100Payload;
type StdoutBytesResultTag = TryType100Tag;
type StdoutIOErr = IOErrType97;
type StdoutIOErrPayload = IOErrType97Payload;
type StdoutIOErrTag = IOErrType97Tag;

extern "C" {
    fn roc_main(args: RocList<RocStr>) -> i32;
}

static DEBUG_OR_EXPECT_CALLED: AtomicBool = AtomicBool::new(false);
static mut ROC_HOST: *mut RocHost = core::ptr::null_mut();

fn set_roc_host(roc_host: *mut RocHost) {
    unsafe {
        ROC_HOST = roc_host;
    }
}

fn roc_host_ptr() -> *mut RocHost {
    unsafe {
        if ROC_HOST.is_null() {
            eprintln!("roc host error: RocHost not initialized");
            std::process::exit(1);
        }
        ROC_HOST
    }
}

fn roc_host() -> &'static RocHost {
    unsafe { &*roc_host_ptr() }
}

macro_rules! define_common_io_err {
    ($from_io:ident, $other:ident, $ty:ident, $tag:ident, $payload:ident) => {
        fn $other(message: &str, roc_host: &RocHost) -> $ty {
            $ty {
                payload: $payload {
                    other: ManuallyDrop::new(RocStr::from_str(message, roc_host)),
                },
                tag: $tag::Other,
            }
        }

        fn $from_io(error: &io::Error, roc_host: &RocHost) -> $ty {
            match error.kind() {
                io::ErrorKind::AlreadyExists => $ty {
                    payload: $payload { already_exists: [] },
                    tag: $tag::AlreadyExists,
                },
                io::ErrorKind::BrokenPipe => $ty {
                    payload: $payload { broken_pipe: [] },
                    tag: $tag::BrokenPipe,
                },
                io::ErrorKind::Interrupted => $ty {
                    payload: $payload { interrupted: [] },
                    tag: $tag::Interrupted,
                },
                io::ErrorKind::NotFound => $ty {
                    payload: $payload { not_found: [] },
                    tag: $tag::NotFound,
                },
                io::ErrorKind::OutOfMemory => $ty {
                    payload: $payload { out_of_memory: [] },
                    tag: $tag::OutOfMemory,
                },
                io::ErrorKind::PermissionDenied => $ty {
                    payload: $payload {
                        permission_denied: [],
                    },
                    tag: $tag::PermissionDenied,
                },
                io::ErrorKind::Unsupported => $ty {
                    payload: $payload { unsupported: [] },
                    tag: $tag::Unsupported,
                },
                _ => $other(&error.to_string(), roc_host),
            }
        }
    };
}

define_common_io_err!(
    cmd_io_err_from_io,
    cmd_io_err_other,
    CmdIOErr,
    CmdIOErrTag,
    CmdIOErrPayload
);
define_common_io_err!(
    dir_io_err_from_io,
    dir_io_err_other,
    DirIOErr,
    DirIOErrTag,
    DirIOErrPayload
);
define_common_io_err!(
    file_io_err_from_io,
    file_io_err_other,
    FileIOErr,
    FileIOErrTag,
    FileIOErrPayload
);
define_common_io_err!(
    path_io_err_from_io,
    path_io_err_other,
    PathIOErr,
    PathIOErrTag,
    PathIOErrPayload
);
define_common_io_err!(
    random_io_err_from_io,
    random_io_err_other,
    RandomIOErr,
    RandomIOErrTag,
    RandomIOErrPayload
);
define_common_io_err!(
    stderr_io_err_from_io,
    stderr_io_err_other,
    StderrIOErr,
    StderrIOErrTag,
    StderrIOErrPayload
);
define_common_io_err!(
    stdin_io_err_from_io,
    stdin_io_err_other,
    StdinIOErr,
    StdinIOErrTag,
    StdinIOErrPayload
);
define_common_io_err!(
    stdout_io_err_from_io,
    stdout_io_err_other,
    StdoutIOErr,
    StdoutIOErrTag,
    StdoutIOErrPayload
);

fn decref_roc_str_list(list: &RocList<RocStr>, roc_host: &RocHost) {
    for item in list.as_slice() {
        item.decref(roc_host);
    }
    list.decref(roc_host);
}

fn decref_host_cmd_arg(cmd: &Cmd, roc_host: &RocHost) {
    decref_roc_str_list(&cmd.args, roc_host);
    decref_roc_str_list(&cmd.envs, roc_host);
    cmd.program.decref(roc_host);
}

fn cmd_to_std(cmd: &Cmd) -> std::process::Command {
    let mut std_cmd = std::process::Command::new(cmd.program.as_str());

    for arg in cmd.args.as_slice() {
        std_cmd.arg(arg.as_str());
    }

    if cmd.clear_envs {
        std_cmd.env_clear();
    }

    for chunk in cmd.envs.as_slice().chunks(2) {
        if let [key, value] = chunk {
            std_cmd.env(key.as_str(), value.as_str());
        }
    }

    std_cmd
}

fn try_cmd_exit_ok(value: i32) -> CmdExitResult {
    CmdExitResult {
        payload: CmdExitResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: CmdExitResultTag::Ok,
    }
}

fn try_cmd_exit_err(error: CmdIOErr) -> CmdExitResult {
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

fn try_cmd_output_err(error: CmdOutputFailureResult) -> CmdOutputResult {
    CmdOutputResult {
        payload: CmdOutputResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: CmdOutputResultTag::Err,
    }
}

fn try_cmd_output_failure_ok(value: CmdOutputFailure) -> CmdOutputFailureResult {
    CmdOutputFailureResult {
        payload: CmdOutputFailureResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: CmdOutputFailureResultTag::Ok,
    }
}

fn try_cmd_output_failure_err(error: CmdIOErr) -> CmdOutputFailureResult {
    CmdOutputFailureResult {
        payload: CmdOutputFailureResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: CmdOutputFailureResultTag::Err,
    }
}

fn try_dir_unit_ok() -> DirUnitResult {
    DirUnitResult {
        payload: DirUnitResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: DirUnitResultTag::Ok,
    }
}

fn try_dir_unit_err(error: DirIOErr) -> DirUnitResult {
    DirUnitResult {
        payload: DirUnitResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: DirUnitResultTag::Err,
    }
}

fn try_dir_list_ok(value: RocList<RocStr>) -> DirListResult {
    DirListResult {
        payload: DirListResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: DirListResultTag::Ok,
    }
}

fn try_dir_list_err(error: DirIOErr) -> DirListResult {
    DirListResult {
        payload: DirListResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: DirListResultTag::Err,
    }
}

fn try_env_str_ok(value: RocStr) -> EnvVarResult {
    EnvVarResult {
        payload: EnvVarResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: EnvVarResultTag::Ok,
    }
}

fn try_env_str_err(error: RocStr) -> EnvVarResult {
    EnvVarResult {
        payload: EnvVarResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: EnvVarResultTag::Err,
    }
}

fn try_env_cwd_ok(value: RocStr) -> EnvCwdResult {
    EnvCwdResult {
        payload: EnvCwdResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: EnvCwdResultTag::Ok,
    }
}

fn try_env_cwd_err() -> EnvCwdResult {
    EnvCwdResult {
        payload: EnvCwdResultPayload {
            err: ManuallyDrop::new(core::ptr::null_mut()),
        },
        tag: EnvCwdResultTag::Err,
    }
}

fn try_env_exe_path_ok(value: RocStr) -> EnvExePathResult {
    EnvExePathResult {
        payload: EnvExePathResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: EnvExePathResultTag::Ok,
    }
}

fn try_env_exe_path_err() -> EnvExePathResult {
    EnvExePathResult {
        payload: EnvExePathResultPayload {
            err: ManuallyDrop::new(core::ptr::null_mut()),
        },
        tag: EnvExePathResultTag::Err,
    }
}

fn try_file_bytes_ok(value: RocListWith<u8, false>) -> FileBytesResult {
    FileBytesResult {
        payload: FileBytesResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: FileBytesResultTag::Ok,
    }
}

fn try_file_bytes_err(error: FileIOErr) -> FileBytesResult {
    FileBytesResult {
        payload: FileBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileBytesResultTag::Err,
    }
}

fn try_file_write_bytes_ok() -> FileWriteBytesResult {
    FileWriteBytesResult {
        payload: FileWriteBytesResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: FileWriteBytesResultTag::Ok,
    }
}

fn try_file_write_bytes_err(error: FileIOErr) -> FileWriteBytesResult {
    FileWriteBytesResult {
        payload: FileWriteBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileWriteBytesResultTag::Err,
    }
}

fn try_file_str_ok(value: RocStr) -> FileStrResult {
    FileStrResult {
        payload: FileStrResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: FileStrResultTag::Ok,
    }
}

fn try_file_str_err(error: FileIOErr) -> FileStrResult {
    FileStrResult {
        payload: FileStrResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileStrResultTag::Err,
    }
}

fn try_file_write_utf8_ok() -> FileWriteUtf8Result {
    FileWriteUtf8Result {
        payload: FileWriteUtf8ResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: FileWriteUtf8ResultTag::Ok,
    }
}

fn try_file_write_utf8_err(error: FileIOErr) -> FileWriteUtf8Result {
    FileWriteUtf8Result {
        payload: FileWriteUtf8ResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileWriteUtf8ResultTag::Err,
    }
}

fn try_file_delete_ok() -> FileDeleteResult {
    FileDeleteResult {
        payload: FileDeleteResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: FileDeleteResultTag::Ok,
    }
}

fn try_file_delete_err(error: FileIOErr) -> FileDeleteResult {
    FileDeleteResult {
        payload: FileDeleteResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileDeleteResultTag::Err,
    }
}

fn try_file_size_ok(value: u64) -> FileSizeResult {
    FileSizeResult {
        payload: FileSizeResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: FileSizeResultTag::Ok,
    }
}

fn try_file_size_err(error: FileIOErr) -> FileSizeResult {
    FileSizeResult {
        payload: FileSizeResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileSizeResultTag::Err,
    }
}

fn try_file_bool_ok(value: bool) -> FileBoolResult {
    FileBoolResult {
        payload: FileBoolResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: FileBoolResultTag::Ok,
    }
}

fn try_file_bool_err(error: FileIOErr) -> FileBoolResult {
    FileBoolResult {
        payload: FileBoolResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileBoolResultTag::Err,
    }
}

fn try_file_time_ok(value: u128) -> FileTimeResult {
    FileTimeResult {
        payload: FileTimeResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: FileTimeResultTag::Ok,
    }
}

fn try_file_time_err(error: FileIOErr) -> FileTimeResult {
    FileTimeResult {
        payload: FileTimeResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileTimeResultTag::Err,
    }
}

fn try_locale_get_ok(value: RocStr) -> LocaleGetResult {
    LocaleGetResult {
        payload: LocaleGetResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: LocaleGetResultTag::Ok,
    }
}

fn try_path_type_ok(value: PathInfo) -> PathTypeResult {
    PathTypeResult {
        payload: PathTypeResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: PathTypeResultTag::Ok,
    }
}

fn try_path_type_err(error: PathIOErr) -> PathTypeResult {
    PathTypeResult {
        payload: PathTypeResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: PathTypeResultTag::Err,
    }
}

fn try_random_u64_ok(value: u64) -> RandomU64Result {
    RandomU64Result {
        payload: RandomU64ResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: RandomU64ResultTag::Ok,
    }
}

fn try_random_u64_err(error: RandomIOErr) -> RandomU64Result {
    RandomU64Result {
        payload: RandomU64ResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: RandomU64ResultTag::Err,
    }
}

fn try_random_u32_ok(value: u32) -> RandomU32Result {
    RandomU32Result {
        payload: RandomU32ResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: RandomU32ResultTag::Ok,
    }
}

fn try_random_u32_err(error: RandomIOErr) -> RandomU32Result {
    RandomU32Result {
        payload: RandomU32ResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: RandomU32ResultTag::Err,
    }
}

fn try_stderr_unit_ok() -> StderrUnitResult {
    StderrUnitResult {
        payload: StderrUnitResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: StderrUnitResultTag::Ok,
    }
}

fn try_stderr_unit_err(error: StderrIOErr) -> StderrUnitResult {
    StderrUnitResult {
        payload: StderrUnitResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StderrUnitResultTag::Err,
    }
}

fn try_stderr_bytes_ok() -> StderrBytesResult {
    StderrBytesResult {
        payload: StderrBytesResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: StderrBytesResultTag::Ok,
    }
}

fn try_stderr_bytes_err(error: StderrIOErr) -> StderrBytesResult {
    StderrBytesResult {
        payload: StderrBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StderrBytesResultTag::Err,
    }
}

fn stdin_line_eof_or_err_eof() -> StdinLineReadErr {
    StdinLineReadErr {
        payload: StdinLineReadErrPayload { end_of_file: [] },
        tag: StdinLineReadErrTag::EndOfFile,
    }
}

fn stdin_line_eof_or_err_io(error: StdinIOErr) -> StdinLineReadErr {
    StdinLineReadErr {
        payload: StdinLineReadErrPayload {
            stdin_err: ManuallyDrop::new(error),
        },
        tag: StdinLineReadErrTag::StdinErr,
    }
}

fn stdin_bytes_eof_or_err_eof() -> StdinBytesReadErr {
    StdinBytesReadErr {
        payload: StdinBytesReadErrPayload { end_of_file: [] },
        tag: StdinBytesReadErrTag::EndOfFile,
    }
}

fn stdin_bytes_eof_or_err_io(error: StdinIOErr) -> StdinBytesReadErr {
    StdinBytesReadErr {
        payload: StdinBytesReadErrPayload {
            stdin_err: ManuallyDrop::new(error),
        },
        tag: StdinBytesReadErrTag::StdinErr,
    }
}

fn try_stdin_line_ok(value: RocStr) -> StdinLineResult {
    StdinLineResult {
        payload: StdinLineResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: StdinLineResultTag::Ok,
    }
}

fn try_stdin_line_err(error: StdinLineReadErr) -> StdinLineResult {
    StdinLineResult {
        payload: StdinLineResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StdinLineResultTag::Err,
    }
}

fn try_stdin_bytes_ok(value: RocListWith<u8, false>) -> StdinBytesResult {
    StdinBytesResult {
        payload: StdinBytesResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: StdinBytesResultTag::Ok,
    }
}

fn try_stdin_bytes_err(error: StdinBytesReadErr) -> StdinBytesResult {
    StdinBytesResult {
        payload: StdinBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StdinBytesResultTag::Err,
    }
}

fn try_stdin_read_to_end_ok(value: RocListWith<u8, false>) -> StdinReadToEndResult {
    StdinReadToEndResult {
        payload: StdinReadToEndResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: StdinReadToEndResultTag::Ok,
    }
}

fn try_stdin_read_to_end_err(error: StdinIOErr) -> StdinReadToEndResult {
    StdinReadToEndResult {
        payload: StdinReadToEndResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StdinReadToEndResultTag::Err,
    }
}

fn try_stdout_unit_ok() -> StdoutUnitResult {
    StdoutUnitResult {
        payload: StdoutUnitResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: StdoutUnitResultTag::Ok,
    }
}

fn try_stdout_unit_err(error: StdoutIOErr) -> StdoutUnitResult {
    StdoutUnitResult {
        payload: StdoutUnitResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StdoutUnitResultTag::Err,
    }
}

fn try_stdout_bytes_ok() -> StdoutBytesResult {
    StdoutBytesResult {
        payload: StdoutBytesResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: StdoutBytesResultTag::Ok,
    }
}

fn try_stdout_bytes_err(error: StdoutIOErr) -> StdoutBytesResult {
    StdoutBytesResult {
        payload: StdoutBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: StdoutBytesResultTag::Err,
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_host_exec_exit_code(cmd: Cmd) -> CmdExitResult {
    let roc_host = roc_host();
    let mut std_cmd = cmd_to_std(&cmd);
    decref_host_cmd_arg(&cmd, roc_host);

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
    let mut std_cmd = cmd_to_std(&cmd);
    decref_host_cmd_arg(&cmd, roc_host);

    match std_cmd.output() {
        Ok(output) => {
            let stdout_bytes = RocListWith::<u8, false>::from_slice(&output.stdout, roc_host);
            let stderr_bytes = RocListWith::<u8, false>::from_slice(&output.stderr, roc_host);

            match output.status.code() {
                Some(0) => try_cmd_output_ok(CmdOutputSuccess {
                    stderr_bytes,
                    stdout_bytes,
                }),
                Some(exit_code) => {
                    try_cmd_output_err(try_cmd_output_failure_ok(CmdOutputFailure {
                        stderr_bytes,
                        stdout_bytes,
                        exit_code,
                    }))
                }
                None => {
                    stdout_bytes.decref(roc_host);
                    stderr_bytes.decref(roc_host);
                    try_cmd_output_err(try_cmd_output_failure_err(cmd_io_err_other(
                        "Process was killed by signal",
                        roc_host,
                    )))
                }
            }
        }
        Err(error) => try_cmd_output_err(try_cmd_output_failure_err(cmd_io_err_from_io(
            &error, roc_host,
        ))),
    }
}

fn path_from_roc_str(path: RocStr, roc_host: &RocHost) -> String {
    let path_string = path.as_str().to_owned();
    path.decref(roc_host);
    path_string
}

#[no_mangle]
pub extern "C" fn hosted_dir_create(path: RocStr) -> DirUnitResult {
    let roc_host = roc_host();
    match fs::create_dir(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_create_all(path: RocStr) -> DirUnitResult {
    let roc_host = roc_host();
    match fs::create_dir_all(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_delete_all(path: RocStr) -> DirUnitResult {
    let roc_host = roc_host();
    match fs::remove_dir_all(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_delete_empty(path: RocStr) -> DirUnitResult {
    let roc_host = roc_host();
    match fs::remove_dir(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_list(path: RocStr) -> DirListResult {
    let roc_host = roc_host();
    match fs::read_dir(path_from_roc_str(path, roc_host)) {
        Ok(read_dir) => {
            let entries: Vec<String> = read_dir
                .filter_map(|entry| {
                    entry
                        .ok()
                        .map(|entry| entry.path().to_string_lossy().into_owned())
                })
                .collect();
            let list = RocList::<RocStr>::allocate(entries.len(), roc_host);
            for (index, entry) in entries.iter().enumerate() {
                unsafe {
                    list.elements
                        .add(index)
                        .write(RocStr::from_str(entry, roc_host));
                }
            }
            try_dir_list_ok(list)
        }
        Err(error) => try_dir_list_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_env_cwd() -> EnvCwdResult {
    let roc_host = roc_host();
    match std::env::current_dir() {
        Ok(path) => try_env_cwd_ok(RocStr::from_str(path.to_string_lossy().as_ref(), roc_host)),
        Err(_) => try_env_cwd_err(),
    }
}

#[no_mangle]
pub extern "C" fn hosted_env_exe_path() -> EnvExePathResult {
    let roc_host = roc_host();
    match std::env::current_exe() {
        Ok(path) => {
            try_env_exe_path_ok(RocStr::from_str(path.to_string_lossy().as_ref(), roc_host))
        }
        Err(_) => try_env_exe_path_err(),
    }
}

#[no_mangle]
pub extern "C" fn hosted_env_temp_dir() -> RocStr {
    let roc_host = roc_host();
    RocStr::from_str(std::env::temp_dir().to_string_lossy().as_ref(), roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_env_var(name: RocStr) -> EnvVarResult {
    let roc_host = roc_host();
    let key = name.as_str().to_owned();
    match std::env::var_os(&key) {
        Some(value) => {
            name.decref(roc_host);
            try_env_str_ok(RocStr::from_str(value.to_string_lossy().as_ref(), roc_host))
        }
        None => try_env_str_err(name),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_delete(path: RocStr) -> FileDeleteResult {
    let roc_host = roc_host();
    match fs::remove_file(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_file_delete_ok(),
        Err(error) => try_file_delete_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_read_bytes(path: RocStr) -> FileBytesResult {
    let roc_host = roc_host();
    match fs::read(path_from_roc_str(path, roc_host)) {
        Ok(bytes) => try_file_bytes_ok(RocListWith::<u8, false>::from_slice(&bytes, roc_host)),
        Err(error) => try_file_bytes_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_read_utf8(path: RocStr) -> FileStrResult {
    let roc_host = roc_host();
    match fs::read_to_string(path_from_roc_str(path, roc_host)) {
        Ok(content) => try_file_str_ok(RocStr::from_str(&content, roc_host)),
        Err(error) => try_file_str_err(file_io_err_from_io(&error, roc_host)),
    }
}

fn file_metadata(path: RocStr, roc_host: &RocHost) -> io::Result<fs::Metadata> {
    fs::metadata(path_from_roc_str(path, roc_host))
}

#[no_mangle]
pub extern "C" fn hosted_file_size_in_bytes(path: RocStr) -> FileSizeResult {
    let roc_host = roc_host();
    match file_metadata(path, roc_host) {
        Ok(metadata) => try_file_size_ok(metadata.len()),
        Err(error) => try_file_size_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[cfg(not(unix))]
fn unsupported_file_permission_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "file permission checks are not implemented on this platform",
    )
}

fn file_permission_bit(path: RocStr, roc_host: &RocHost, bit: u32) -> io::Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = file_metadata(path, roc_host)?;
        Ok(metadata.permissions().mode() & bit != 0)
    }

    #[cfg(not(unix))]
    {
        let _ = path_from_roc_str(path, roc_host);
        let _ = bit;
        Err(unsupported_file_permission_error())
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_is_executable(path: RocStr) -> FileBoolResult {
    let roc_host = roc_host();
    match file_permission_bit(path, roc_host, 0o111) {
        Ok(value) => try_file_bool_ok(value),
        Err(error) => try_file_bool_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_is_readable(path: RocStr) -> FileBoolResult {
    let roc_host = roc_host();
    match file_permission_bit(path, roc_host, 0o400) {
        Ok(value) => try_file_bool_ok(value),
        Err(error) => try_file_bool_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_is_writable(path: RocStr) -> FileBoolResult {
    let roc_host = roc_host();
    match file_permission_bit(path, roc_host, 0o200) {
        Ok(value) => try_file_bool_ok(value),
        Err(error) => try_file_bool_err(file_io_err_from_io(&error, roc_host)),
    }
}

fn nanos_since_epoch(time: std::time::SystemTime) -> io::Result<u128> {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))
}

fn file_time(
    path: RocStr,
    roc_host: &RocHost,
    read_time: fn(&fs::Metadata) -> io::Result<std::time::SystemTime>,
) -> io::Result<u128> {
    let metadata = file_metadata(path, roc_host)?;
    read_time(&metadata).and_then(nanos_since_epoch)
}

#[no_mangle]
pub extern "C" fn hosted_file_time_accessed(path: RocStr) -> FileTimeResult {
    let roc_host = roc_host();
    match file_time(path, roc_host, fs::Metadata::accessed) {
        Ok(value) => try_file_time_ok(value),
        Err(error) => try_file_time_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_time_created(path: RocStr) -> FileTimeResult {
    let roc_host = roc_host();
    match file_time(path, roc_host, fs::Metadata::created) {
        Ok(value) => try_file_time_ok(value),
        Err(error) => try_file_time_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_time_modified(path: RocStr) -> FileTimeResult {
    let roc_host = roc_host();
    match file_time(path, roc_host, fs::Metadata::modified) {
        Ok(value) => try_file_time_ok(value),
        Err(error) => try_file_time_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_write_bytes(
    path: RocStr,
    bytes: RocListWith<u8, false>,
) -> FileWriteBytesResult {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    let result = fs::write(path_string, bytes.as_slice());
    bytes.decref(roc_host);

    match result {
        Ok(()) => try_file_write_bytes_ok(),
        Err(error) => try_file_write_bytes_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_write_utf8(path: RocStr, content: RocStr) -> FileWriteUtf8Result {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    let content_string = content.as_str().to_owned();
    content.decref(roc_host);

    match fs::write(path_string, content_string) {
        Ok(()) => try_file_write_utf8_ok(),
        Err(error) => try_file_write_utf8_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[cfg(target_os = "macos")]
fn locale_from_env() -> Option<String> {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }

            let locale = trimmed
                .split('.')
                .next()
                .unwrap_or(trimmed)
                .split('@')
                .next()
                .unwrap_or(trimmed)
                .trim();

            if !locale.is_empty() {
                return Some(locale.to_string());
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn locale_get_string() -> String {
    locale_from_env().unwrap_or_else(|| "en-US".to_string())
}

#[cfg(not(target_os = "macos"))]
fn locale_get_string() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
}

#[cfg(target_os = "macos")]
fn locale_all_strings() -> Vec<String> {
    vec![locale_get_string()]
}

#[cfg(not(target_os = "macos"))]
fn locale_all_strings() -> Vec<String> {
    let locales = sys_locale::get_locales().collect::<Vec<_>>();
    if locales.is_empty() {
        vec![locale_get_string()]
    } else {
        locales
    }
}

#[no_mangle]
pub extern "C" fn hosted_locale_all() -> RocList<RocStr> {
    let roc_host = roc_host();
    let locales = locale_all_strings();
    let list = RocList::<RocStr>::allocate(locales.len(), roc_host);

    for (index, locale) in locales.iter().enumerate() {
        unsafe {
            list.elements
                .add(index)
                .write(RocStr::from_str(locale, roc_host));
        }
    }

    list
}

#[no_mangle]
pub extern "C" fn hosted_locale_get() -> LocaleGetResult {
    let roc_host = roc_host();
    try_locale_get_ok(RocStr::from_str(&locale_get_string(), roc_host))
}

fn path_buf_from_roc_bytes(
    bytes: RocListWith<u8, false>,
    roc_host: &RocHost,
) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let path = std::ffi::OsStr::from_bytes(bytes.as_slice()).to_owned();
        bytes.decref(roc_host);
        std::path::PathBuf::from(path)
    }

    #[cfg(not(unix))]
    {
        let path = String::from_utf8_lossy(bytes.as_slice()).into_owned();
        bytes.decref(roc_host);
        std::path::PathBuf::from(path)
    }
}

#[no_mangle]
pub extern "C" fn hosted_path_type(path: RocListWith<u8, false>) -> PathTypeResult {
    let roc_host = roc_host();
    let path = path_buf_from_roc_bytes(path, roc_host);

    match path.symlink_metadata() {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            try_path_type_ok(PathInfo {
                is_dir: metadata.is_dir(),
                is_file: metadata.is_file(),
                is_sym_link: file_type.is_symlink(),
            })
        }
        Err(error) => try_path_type_err(path_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_random_seed_u32() -> RandomU32Result {
    let roc_host = roc_host();
    let mut bytes = [0u8; 4];
    match getrandom::getrandom(&mut bytes) {
        Ok(()) => try_random_u32_ok(u32::from_ne_bytes(bytes)),
        Err(error) => {
            let io_error = io::Error::new(io::ErrorKind::Other, error.to_string());
            try_random_u32_err(random_io_err_from_io(&io_error, roc_host))
        }
    }
}

#[no_mangle]
pub extern "C" fn hosted_random_seed_u64() -> RandomU64Result {
    let roc_host = roc_host();
    let mut bytes = [0u8; 8];
    match getrandom::getrandom(&mut bytes) {
        Ok(()) => try_random_u64_ok(u64::from_ne_bytes(bytes)),
        Err(error) => {
            let io_error = io::Error::new(io::ErrorKind::Other, error.to_string());
            try_random_u64_err(random_io_err_from_io(&io_error, roc_host))
        }
    }
}

#[no_mangle]
pub extern "C" fn hosted_sleep_millis(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

#[no_mangle]
pub extern "C" fn hosted_stderr_line(message: RocStr) -> StderrUnitResult {
    let roc_host = roc_host();
    let result = {
        let mut stderr = io::stderr().lock();
        writeln!(stderr, "{}", message.as_str())
    };
    message.decref(roc_host);

    match result {
        Ok(()) => try_stderr_unit_ok(),
        Err(error) => try_stderr_unit_err(stderr_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stderr_write(message: RocStr) -> StderrUnitResult {
    let roc_host = roc_host();
    let result = {
        let mut stderr = io::stderr().lock();
        write!(stderr, "{}", message.as_str()).and_then(|()| stderr.flush())
    };
    message.decref(roc_host);

    match result {
        Ok(()) => try_stderr_unit_ok(),
        Err(error) => try_stderr_unit_err(stderr_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stderr_write_bytes(bytes: RocListWith<u8, false>) -> StderrBytesResult {
    let roc_host = roc_host();
    let result = {
        let mut stderr = io::stderr().lock();
        stderr
            .write_all(bytes.as_slice())
            .and_then(|()| stderr.flush())
    };
    bytes.decref(roc_host);

    match result {
        Ok(()) => try_stderr_bytes_ok(),
        Err(error) => try_stderr_bytes_err(stderr_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdin_line() -> StdinLineResult {
    let roc_host = roc_host();
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) => try_stdin_line_err(stdin_line_eof_or_err_eof()),
        Ok(_) => {
            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
            try_stdin_line_ok(RocStr::from_str(trimmed, roc_host))
        }
        Err(error) => try_stdin_line_err(stdin_line_eof_or_err_io(stdin_io_err_from_io(
            &error, roc_host,
        ))),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdin_bytes() -> StdinBytesResult {
    let roc_host = roc_host();
    let mut buffer = [0u8; 16_384];
    match io::stdin().lock().read(&mut buffer) {
        Ok(0) => try_stdin_bytes_err(stdin_bytes_eof_or_err_eof()),
        Ok(bytes_read) => try_stdin_bytes_ok(RocListWith::<u8, false>::from_slice(
            &buffer[..bytes_read],
            roc_host,
        )),
        Err(error) => try_stdin_bytes_err(stdin_bytes_eof_or_err_io(stdin_io_err_from_io(
            &error, roc_host,
        ))),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdin_read_to_end() -> StdinReadToEndResult {
    let roc_host = roc_host();
    let mut buffer = Vec::new();
    match io::stdin().lock().read_to_end(&mut buffer) {
        Ok(_) => try_stdin_read_to_end_ok(RocListWith::<u8, false>::from_slice(&buffer, roc_host)),
        Err(error) => try_stdin_read_to_end_err(stdin_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdout_line(message: RocStr) -> StdoutUnitResult {
    let roc_host = roc_host();
    let result = {
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{}", message.as_str())
    };
    message.decref(roc_host);

    match result {
        Ok(()) => try_stdout_unit_ok(),
        Err(error) => try_stdout_unit_err(stdout_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdout_write(message: RocStr) -> StdoutUnitResult {
    let roc_host = roc_host();
    let result = {
        let mut stdout = io::stdout().lock();
        write!(stdout, "{}", message.as_str()).and_then(|()| stdout.flush())
    };
    message.decref(roc_host);

    match result {
        Ok(()) => try_stdout_unit_ok(),
        Err(error) => try_stdout_unit_err(stdout_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdout_write_bytes(bytes: RocListWith<u8, false>) -> StdoutBytesResult {
    let roc_host = roc_host();
    let result = {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(bytes.as_slice())
            .and_then(|()| stdout.flush())
    };
    bytes.decref(roc_host);

    match result {
        Ok(()) => try_stdout_bytes_ok(),
        Err(error) => try_stdout_bytes_err(stdout_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_tty_disable_raw_mode() {
    let _ = disable_raw_mode();
}

#[no_mangle]
pub extern "C" fn hosted_tty_enable_raw_mode() {
    let _ = enable_raw_mode();
}

#[no_mangle]
pub extern "C" fn hosted_utc_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos()
}

#[no_mangle]
pub extern "C" fn roc_alloc(length: usize, alignment: usize) -> *mut c_void {
    DefaultAllocators::roc_alloc(roc_host_ptr(), length, alignment)
}

#[no_mangle]
pub extern "C" fn roc_dealloc(ptr: *mut c_void, alignment: usize) {
    DefaultAllocators::roc_dealloc(roc_host_ptr(), ptr, alignment);
}

#[no_mangle]
pub extern "C" fn roc_realloc(
    ptr: *mut c_void,
    new_length: usize,
    alignment: usize,
) -> *mut c_void {
    DefaultAllocators::roc_realloc(roc_host_ptr(), ptr, new_length, alignment)
}

#[no_mangle]
pub extern "C" fn roc_dbg(bytes: *const u8, len: usize) {
    DEBUG_OR_EXPECT_CALLED.store(true, Ordering::Release);
    DefaultHandlers::roc_dbg(roc_host_ptr(), bytes, len);
}

#[no_mangle]
pub extern "C" fn roc_expect_failed(bytes: *const u8, len: usize) {
    DEBUG_OR_EXPECT_CALLED.store(true, Ordering::Release);
    DefaultHandlers::roc_expect_failed(roc_host_ptr(), bytes, len);
}

#[no_mangle]
pub extern "C" fn roc_crashed(bytes: *const u8, len: usize) {
    DefaultHandlers::roc_crashed(roc_host_ptr(), bytes, len);
}

fn build_args_list(argc: i32, argv: *const *const c_char, roc_host: &RocHost) -> RocList<RocStr> {
    if argc <= 0 || argv.is_null() {
        return RocList::empty();
    }

    let list = RocList::<RocStr>::allocate(argc as usize, roc_host);
    for index in 0..argc as isize {
        unsafe {
            let arg_ptr = *argv.offset(index);
            if arg_ptr.is_null() {
                break;
            }
            let arg = CStr::from_ptr(arg_ptr).to_string_lossy();
            list.elements
                .offset(index)
                .write(RocStr::from_str(&arg, roc_host));
        }
    }
    list
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const c_char) -> i32 {
    rust_main(argc, argv)
}

pub fn rust_main(argc: i32, argv: *const *const c_char) -> i32 {
    let mut roc_host = make_roc_host(core::ptr::null_mut());
    set_roc_host(&mut roc_host);

    let args_list = build_args_list(argc, argv, &roc_host);
    let mut exit_code = unsafe { roc_main(args_list) };

    if DEBUG_OR_EXPECT_CALLED.load(Ordering::Acquire) && exit_code == 0 {
        exit_code = 1;
    }

    set_roc_host(core::ptr::null_mut());
    exit_code
}

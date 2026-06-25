//! Roc platform host implementation for Roc's direct-symbol host ABI.

#![allow(improper_ctypes_definitions)]

use core::mem::ManuallyDrop;
use std::ffi::{c_char, c_void, CStr};
use std::fs;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

mod roc_platform_abi;

use crate::roc_platform_abi::*;

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

fn dir_io_err_other(message: &str, roc_host: &RocHost) -> DirIOErr {
    DirIOErr {
        payload: DirIOErrPayload {
            other: ManuallyDrop::new(RocStr::from_str(message, roc_host)),
        },
        tag: DirIOErrTag::Other,
    }
}

fn dir_io_err_from_io(error: &io::Error, roc_host: &RocHost) -> DirIOErr {
    if let Some(errno) = error.raw_os_error() {
        if errno == libc::ENOTDIR {
            return DirIOErr {
                payload: DirIOErrPayload { not_adirectory: [] },
                tag: DirIOErrTag::NotADirectory,
            };
        }
        if errno == libc::ENOTEMPTY {
            return DirIOErr {
                payload: DirIOErrPayload { not_empty: [] },
                tag: DirIOErrTag::NotEmpty,
            };
        }
    }

    match error.kind() {
        io::ErrorKind::AlreadyExists => DirIOErr {
            payload: DirIOErrPayload { already_exists: [] },
            tag: DirIOErrTag::AlreadyExists,
        },
        io::ErrorKind::NotFound => DirIOErr {
            payload: DirIOErrPayload { not_found: [] },
            tag: DirIOErrTag::NotFound,
        },
        io::ErrorKind::PermissionDenied => DirIOErr {
            payload: DirIOErrPayload {
                permission_denied: [],
            },
            tag: DirIOErrTag::PermissionDenied,
        },
        _ => dir_io_err_other(&error.to_string(), roc_host),
    }
}

fn decref_roc_str_list(list: &RocList<RocStr>, roc_host: &RocHost) {
    for item in list.as_slice() {
        item.decref(roc_host);
    }
    list.decref(roc_host);
}

fn decref_cmd(cmd: &Cmd, roc_host: &RocHost) {
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

fn roc_str_lossy(bytes: &[u8], roc_host: &RocHost) -> RocStr {
    RocStr::from_str(String::from_utf8_lossy(bytes).as_ref(), roc_host)
}

fn try_cmd_exit_ok(value: i32) -> TryType0 {
    TryType0 {
        payload: TryType0Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType0Tag::Ok,
    }
}

fn try_cmd_exit_err(error: CmdIOErr) -> TryType0 {
    TryType0 {
        payload: TryType0Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType0Tag::Err,
    }
}

fn try_cmd_output_ok(value: AnonStruct11) -> TryType8 {
    TryType8 {
        payload: TryType8Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType8Tag::Ok,
    }
}

fn try_cmd_output_err(error: CmdErrOrNonZeroExit) -> TryType8 {
    TryType8 {
        payload: TryType8Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType8Tag::Err,
    }
}

fn try_dir_unit_ok() -> TryType12 {
    TryType12 {
        payload: TryType12Payload {
            ok: ManuallyDrop::new(()),
        },
        tag: TryType12Tag::Ok,
    }
}

fn try_dir_unit_err(error: DirIOErr) -> TryType12 {
    TryType12 {
        payload: TryType12Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType12Tag::Err,
    }
}

fn try_dir_list_ok(value: RocList<RocStr>) -> TryType17 {
    TryType17 {
        payload: TryType17Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType17Tag::Ok,
    }
}

fn try_dir_list_err(error: DirIOErr) -> TryType17 {
    TryType17 {
        payload: TryType17Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType17Tag::Err,
    }
}

fn try_file_bytes_ok(value: RocListWith<u8, false>) -> TryType21 {
    TryType21 {
        payload: TryType21Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType21Tag::Ok,
    }
}

fn try_file_bytes_err(error: FileIOErr) -> TryType21 {
    TryType21 {
        payload: TryType21Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType21Tag::Err,
    }
}

fn try_file_unit_ok() -> TryType27 {
    TryType27 {
        payload: TryType27Payload {
            ok: ManuallyDrop::new(()),
        },
        tag: TryType27Tag::Ok,
    }
}

fn try_file_unit_err(error: FileIOErr) -> TryType27 {
    TryType27 {
        payload: TryType27Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType27Tag::Err,
    }
}

fn try_file_str_ok(value: RocStr) -> TryType29 {
    TryType29 {
        payload: TryType29Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType29Tag::Ok,
    }
}

fn try_file_str_err(error: FileIOErr) -> TryType29 {
    TryType29 {
        payload: TryType29Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType29Tag::Err,
    }
}

fn try_path_bool_ok(value: bool) -> TryType32 {
    TryType32 {
        payload: TryType32Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType32Tag::Ok,
    }
}

fn try_path_bool_err(error: PathIOErr) -> TryType32 {
    TryType32 {
        payload: TryType32Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType32Tag::Err,
    }
}

fn try_random_u64_ok(value: u64) -> TryType37 {
    TryType37 {
        payload: TryType37Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType37Tag::Ok,
    }
}

fn try_random_u64_err(error: RandomIOErr) -> TryType37 {
    TryType37 {
        payload: TryType37Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType37Tag::Err,
    }
}

fn try_random_u32_ok(value: u32) -> TryType43 {
    TryType43 {
        payload: TryType43Payload {
            ok: ManuallyDrop::new(value),
        },
        tag: TryType43Tag::Ok,
    }
}

fn try_random_u32_err(error: RandomIOErr) -> TryType43 {
    TryType43 {
        payload: TryType43Payload {
            err: ManuallyDrop::new(error),
        },
        tag: TryType43Tag::Err,
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_exec_exit_code(cmd: Cmd) -> TryType0 {
    let roc_host = roc_host();
    let mut std_cmd = cmd_to_std(&cmd);
    decref_cmd(&cmd, roc_host);

    match std_cmd.status() {
        Ok(status) => match status.code() {
            Some(code) => try_cmd_exit_ok(code),
            None => try_cmd_exit_err(cmd_io_err_other("Process was killed by signal", roc_host)),
        },
        Err(error) => try_cmd_exit_err(cmd_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_cmd_exec_output(cmd: Cmd) -> TryType8 {
    let roc_host = roc_host();
    let mut std_cmd = cmd_to_std(&cmd);
    decref_cmd(&cmd, roc_host);

    match std_cmd.output() {
        Ok(output) => {
            let stdout = roc_str_lossy(&output.stdout, roc_host);
            let stderr = roc_str_lossy(&output.stderr, roc_host);

            match output.status.code() {
                Some(0) => try_cmd_output_ok(AnonStruct11 {
                    stderr_utf8_lossy: stderr,
                    stdout_utf8: stdout,
                }),
                Some(exit_code) => try_cmd_output_err(CmdErrOrNonZeroExit {
                    payload: CmdErrOrNonZeroExitPayload {
                        non_zero_exit: ManuallyDrop::new(AnonStruct10 {
                            stderr_utf8_lossy: stderr,
                            stdout_utf8_lossy: stdout,
                            exit_code,
                        }),
                    },
                    tag: CmdErrOrNonZeroExitTag::NonZeroExit,
                }),
                None => {
                    stdout.decref(roc_host);
                    stderr.decref(roc_host);
                    try_cmd_output_err(CmdErrOrNonZeroExit {
                        payload: CmdErrOrNonZeroExitPayload {
                            cmd_err: ManuallyDrop::new(cmd_io_err_other(
                                "Process was killed by signal",
                                roc_host,
                            )),
                        },
                        tag: CmdErrOrNonZeroExitTag::CmdErr,
                    })
                }
            }
        }
        Err(error) => try_cmd_output_err(CmdErrOrNonZeroExit {
            payload: CmdErrOrNonZeroExitPayload {
                cmd_err: ManuallyDrop::new(cmd_io_err_from_io(&error, roc_host)),
            },
            tag: CmdErrOrNonZeroExitTag::CmdErr,
        }),
    }
}

fn path_from_roc_str(path: RocStr, roc_host: &RocHost) -> String {
    let path_string = path.as_str().to_owned();
    path.decref(roc_host);
    path_string
}

#[no_mangle]
pub extern "C" fn hosted_dir_create(path: RocStr) -> TryType12 {
    let roc_host = roc_host();
    match fs::create_dir(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_create_all(path: RocStr) -> TryType12 {
    let roc_host = roc_host();
    match fs::create_dir_all(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_delete_all(path: RocStr) -> TryType12 {
    let roc_host = roc_host();
    match fs::remove_dir_all(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_delete_empty(path: RocStr) -> TryType12 {
    let roc_host = roc_host();
    match fs::remove_dir(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_dir_unit_ok(),
        Err(error) => try_dir_unit_err(dir_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_dir_list(path: RocStr) -> TryType17 {
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
pub extern "C" fn hosted_env_cwd() -> RocStr {
    let roc_host = roc_host();
    let cwd = std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    RocStr::from_str(&cwd, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_env_exe_path() -> RocStr {
    let roc_host = roc_host();
    let exe_path = std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    RocStr::from_str(&exe_path, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_env_var(name: RocStr) -> RocStr {
    let roc_host = roc_host();
    let value = std::env::var(name.as_str()).unwrap_or_default();
    name.decref(roc_host);
    RocStr::from_str(&value, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_file_delete(path: RocStr) -> TryType27 {
    let roc_host = roc_host();
    match fs::remove_file(path_from_roc_str(path, roc_host)) {
        Ok(()) => try_file_unit_ok(),
        Err(error) => try_file_unit_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_read_bytes(path: RocStr) -> TryType21 {
    let roc_host = roc_host();
    match fs::read(path_from_roc_str(path, roc_host)) {
        Ok(bytes) => try_file_bytes_ok(RocListWith::<u8, false>::from_slice(&bytes, roc_host)),
        Err(error) => try_file_bytes_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_read_utf8(path: RocStr) -> TryType29 {
    let roc_host = roc_host();
    match fs::read_to_string(path_from_roc_str(path, roc_host)) {
        Ok(content) => try_file_str_ok(RocStr::from_str(&content, roc_host)),
        Err(error) => try_file_str_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_write_bytes(
    path: RocStr,
    bytes: RocListWith<u8, false>,
) -> TryType27 {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    let result = fs::write(path_string, bytes.as_slice());
    bytes.decref(roc_host);

    match result {
        Ok(()) => try_file_unit_ok(),
        Err(error) => try_file_unit_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_write_utf8(path: RocStr, content: RocStr) -> TryType27 {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    let content_string = content.as_str().to_owned();
    content.decref(roc_host);

    match fs::write(path_string, content_string) {
        Ok(()) => try_file_unit_ok(),
        Err(error) => try_file_unit_err(file_io_err_from_io(&error, roc_host)),
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
pub extern "C" fn hosted_locale_get() -> RocStr {
    let roc_host = roc_host();
    RocStr::from_str(&locale_get_string(), roc_host)
}

fn path_bool_result(
    path: RocStr,
    check: impl FnOnce(&std::path::Path) -> io::Result<bool>,
) -> TryType32 {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    match check(std::path::Path::new(&path_string)) {
        Ok(value) => try_path_bool_ok(value),
        Err(error) => try_path_bool_err(path_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_path_is_dir(path: RocStr) -> TryType32 {
    path_bool_result(path, |path| {
        path.symlink_metadata().map(|metadata| metadata.is_dir())
    })
}

#[no_mangle]
pub extern "C" fn hosted_path_is_file(path: RocStr) -> TryType32 {
    path_bool_result(path, |path| {
        path.symlink_metadata().map(|metadata| metadata.is_file())
    })
}

#[no_mangle]
pub extern "C" fn hosted_path_is_sym_link(path: RocStr) -> TryType32 {
    path_bool_result(path, |path| {
        path.symlink_metadata()
            .map(|metadata| metadata.file_type().is_symlink())
    })
}

#[no_mangle]
pub extern "C" fn hosted_random_seed_u32() -> TryType43 {
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
pub extern "C" fn hosted_random_seed_u64() -> TryType37 {
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
pub extern "C" fn hosted_stderr_line(message: RocStr) {
    let roc_host = roc_host();
    let _ = writeln!(io::stderr(), "{}", message.as_str());
    message.decref(roc_host);
}

#[no_mangle]
pub extern "C" fn hosted_stderr_write(message: RocStr) {
    let roc_host = roc_host();
    let _ = write!(io::stderr(), "{}", message.as_str());
    let _ = io::stderr().flush();
    message.decref(roc_host);
}

#[no_mangle]
pub extern "C" fn hosted_stdin_line() -> RocStr {
    let roc_host = roc_host();
    let mut line = String::new();
    let _ = io::stdin().lock().read_line(&mut line);
    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
    RocStr::from_str(trimmed, roc_host)
}

#[no_mangle]
pub extern "C" fn hosted_stdout_line(message: RocStr) {
    let roc_host = roc_host();
    let _ = writeln!(io::stdout(), "{}", message.as_str());
    message.decref(roc_host);
}

#[no_mangle]
pub extern "C" fn hosted_stdout_write(message: RocStr) {
    let roc_host = roc_host();
    let _ = write!(io::stdout(), "{}", message.as_str());
    let _ = io::stdout().flush();
    message.decref(roc_host);
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

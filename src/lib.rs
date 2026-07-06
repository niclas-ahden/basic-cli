//! Roc platform host implementation for Roc's direct-symbol host ABI.

#![allow(improper_ctypes_definitions)]

use core::mem::ManuallyDrop;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

mod roc_platform_abi;

use crate::roc_platform_abi::*;

// RustGlue assigns numbered names (TryTypeN, IOErrTypeN, ...) to anonymous Roc
// records and result types, and the numbers shift whenever a module is added.
// To stay robust against that renumbering we alias against the *semantic* names
// the generator also emits (e.g. `HostCmdExecExitCodeResult`), which are keyed by
// module + function name and therefore stable. Where our preferred local name is
// identical to a generated semantic alias (e.g. `HostIOErr`, `HostDirListResult`), we
// omit it here and rely on the `use crate::roc_platform_abi::*;` glob above.
type CmdExitResult = HostCmdExecExitCodeResult;
type CmdExitResultPayload = HostCmdExecExitCodeResultPayload;
type CmdExitResultTag = HostCmdExecExitCodeResultTag;
type CmdOutputResult = HostCmdExecOutputResult;
type CmdOutputResultPayload = HostCmdExecOutputResultPayload;
type CmdOutputResultTag = HostCmdExecOutputResultTag;
type CmdOutputFailureResult = HostCmdExecOutputErrResult;
type CmdOutputFailureResultPayload = HostCmdExecOutputErrResultPayload;
type CmdOutputFailureResultTag = HostCmdExecOutputErrResultTag;
type CmdOutputFailure = HostCmdExecOutputErrOk;
type CmdOutputSuccess = HostCmdExecOutputOk;
type Cmd = HostCmdExecExitCodeArgs;

type DirUnitResult = HostDirCreateResult;
type DirUnitResultPayload = HostDirCreateResultPayload;
type DirUnitResultTag = HostDirCreateResultTag;

type FileBytesResult = HostFileReadBytesResult;
type FileBytesResultPayload = HostFileReadBytesResultPayload;
type FileBytesResultTag = HostFileReadBytesResultTag;
type FileReaderOpenResult = HostFileOpenReaderResult;
type FileReaderOpenResultPayload = HostFileOpenReaderResultPayload;
type FileReaderOpenResultTag = HostFileOpenReaderResultTag;
type FileReaderLineResult = HostFileReadLineResult;
type FileReaderLineResultPayload = HostFileReadLineResultPayload;
type FileReaderLineResultTag = HostFileReadLineResultTag;
type FileStrResult = HostFileReadUtf8Result;
type FileStrResultPayload = HostFileReadUtf8ResultPayload;
type FileStrResultTag = HostFileReadUtf8ResultTag;
type FileSizeResult = HostFileSizeInBytesResult;
type FileSizeResultPayload = HostFileSizeInBytesResultPayload;
type FileSizeResultTag = HostFileSizeInBytesResultTag;
type FileBoolResult = HostFileIsExecutableResult;
type FileBoolResultPayload = HostFileIsExecutableResultPayload;
type FileBoolResultTag = HostFileIsExecutableResultTag;
type FileTimeResult = HostFileTimeAccessedResult;
type FileTimeResultPayload = HostFileTimeAccessedResultPayload;
type FileTimeResultTag = HostFileTimeAccessedResultTag;

type RandomU64Result = HostRandomSeedU64Result;
type RandomU64ResultPayload = HostRandomSeedU64ResultPayload;
type RandomU64ResultTag = HostRandomSeedU64ResultTag;
type RandomU32Result = HostRandomSeedU32Result;
type RandomU32ResultPayload = HostRandomSeedU32ResultPayload;
type RandomU32ResultTag = HostRandomSeedU32ResultTag;

type StderrUnitResult = HostStderrLineResult;
type StderrUnitResultPayload = HostStderrLineResultPayload;
type StderrUnitResultTag = HostStderrLineResultTag;
type StderrBytesResult = HostStderrWriteBytesResult;
type StderrBytesResultPayload = HostStderrWriteBytesResultPayload;
type StderrBytesResultTag = HostStderrWriteBytesResultTag;

type StdinLineReadErr = EndOfFileOrStdinErr;
type StdinLineReadErrPayload = EndOfFileOrStdinErrPayload;
type StdinLineReadErrTag = EndOfFileOrStdinErrTag;
type StdinBytesReadErr = EndOfFileOrStdinErr;
type StdinBytesReadErrPayload = EndOfFileOrStdinErrPayload;
type StdinBytesReadErrTag = EndOfFileOrStdinErrTag;

type StdoutUnitResult = HostStdoutLineResult;
type StdoutUnitResultPayload = HostStdoutLineResultPayload;
type StdoutUnitResultTag = HostStdoutLineResultTag;
type StdoutBytesResult = HostStdoutWriteBytesResult;
type StdoutBytesResultPayload = HostStdoutWriteBytesResultPayload;
type StdoutBytesResultTag = HostStdoutWriteBytesResultTag;

fn roc_u8_list_from_slice(slice: &[u8], roc_host: &RocHost) -> RocListWith<u8, false> {
    unsafe { RocListWith::<u8, false>::from_slice(slice, roc_host) }
}

// ============================================================================
// SQLite
//
// The generated glue represents `Sqlite.Stmt` (a `Box(U64)`) as `*mut u64`: a
// boxed u64 whose value we use to stash a raw `*mut SqliteStatement`. The box is
// allocated/refcounted with the generated `allocate_box`/`decref_box_with`
// helpers; teardown (running `sqlite3_finalize`) happens in `drop_sqlite_stmt`
// when the last reference is released. Each host fn that takes a handle calls
// `release_sqlite_stmt` before returning to balance the incref Roc performs when
// the value stays live.
// ----------------------------------------------------------------------------

// Generated value/error/state types (see src/roc_platform_abi.rs).
type SqliteValue = BytesOrIntegerOrNullOrRealOrString;
type SqliteValueTag = BytesOrIntegerOrNullOrRealOrStringTag;
type SqliteValuePayload = BytesOrIntegerOrNullOrRealOrStringPayload;
type SqliteError = HostSqlitePrepareErr;
type SqliteBindings = AnonStruct55;

const SQLITE_STMT_BOX_ALIGN: usize = core::mem::align_of::<u64>();

struct SqliteStatement {
    connection: *mut libsqlite3_sys::sqlite3,
    stmt: *mut libsqlite3_sys::sqlite3_stmt,
}

impl Drop for SqliteStatement {
    fn drop(&mut self) {
        unsafe {
            libsqlite3_sys::sqlite3_finalize(self.stmt);
        }
    }
}

thread_local! {
    // Connections are cached per database path and live until process exit.
    static SQLITE_CONNECTIONS: RefCell<Vec<(CString, *mut libsqlite3_sys::sqlite3)>> =
        const { RefCell::new(Vec::new()) };
}

fn box_sqlite_stmt(stmt: SqliteStatement, roc_host: &RocHost) -> *mut u64 {
    let raw: *mut SqliteStatement = Box::into_raw(Box::new(stmt));
    let boxed = unsafe {
        allocate_box(
            core::mem::size_of::<u64>(),
            SQLITE_STMT_BOX_ALIGN,
            false,
            roc_host,
        )
    };
    unsafe {
        *(boxed as *mut u64) = raw as u64;
    }
    boxed as *mut u64
}

unsafe fn sqlite_stmt_ref<'a>(handle: *mut u64) -> &'a mut SqliteStatement {
    &mut *(*handle as *mut SqliteStatement)
}

extern "C" fn drop_sqlite_stmt(data_ptr: *mut c_void, _roc_host: *mut RocHost) {
    unsafe {
        let raw = *(data_ptr as *mut u64) as *mut SqliteStatement;
        if !raw.is_null() {
            drop(Box::from_raw(raw));
        }
    }
}

fn release_sqlite_stmt(handle: *mut u64, roc_host: &RocHost) {
    unsafe {
        decref_box_with(
            handle as RocBox,
            SQLITE_STMT_BOX_ALIGN,
            false,
            Some(drop_sqlite_stmt),
            roc_host,
        );
    }
}

// SQLITE_TRANSIENT tells SQLite to make its own copy of bound text/blob data, so
// we don't have to keep the Roc-owned bytes alive past the bind call.
fn sqlite_transient() -> Option<unsafe extern "C" fn(*mut c_void)> {
    Some(unsafe {
        core::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut c_void)>(
            -1isize as *const c_void,
        )
    })
}

fn sqlite_errmsg(connection: *mut libsqlite3_sys::sqlite3, code: c_int) -> String {
    unsafe {
        let mut message = CStr::from_ptr(libsqlite3_sys::sqlite3_errstr(code))
            .to_string_lossy()
            .into_owned();
        if !connection.is_null() {
            let detailed = libsqlite3_sys::sqlite3_errmsg(connection);
            if !detailed.is_null() {
                message = CStr::from_ptr(detailed).to_string_lossy().into_owned();
            }
        }
        message
    }
}

fn sqlite_error(code: c_int, message: &str, roc_host: &RocHost) -> SqliteError {
    SqliteError {
        code: code as i64,
        message: RocStr::from_str(message, roc_host),
    }
}

fn sqlite_err_from_stmt(stmt: &SqliteStatement, code: c_int, roc_host: &RocHost) -> SqliteError {
    let message = sqlite_errmsg(stmt.connection, code);
    sqlite_error(code, &message, roc_host)
}

fn sqlite_get_connection(path: &str) -> Result<*mut libsqlite3_sys::sqlite3, (c_int, String)> {
    SQLITE_CONNECTIONS.with(|cell| {
        for (conn_path, connection) in cell.borrow().iter() {
            if conn_path.as_bytes() == path.as_bytes() {
                return Ok(*connection);
            }
        }

        let cpath = CString::new(path).map_err(|_| {
            (
                libsqlite3_sys::SQLITE_ERROR,
                "database path contained an interior nul byte".to_string(),
            )
        })?;
        let mut connection: *mut libsqlite3_sys::sqlite3 = core::ptr::null_mut();
        let flags = libsqlite3_sys::SQLITE_OPEN_CREATE
            | libsqlite3_sys::SQLITE_OPEN_READWRITE
            | libsqlite3_sys::SQLITE_OPEN_NOMUTEX;
        let err = unsafe {
            libsqlite3_sys::sqlite3_open_v2(
                cpath.as_ptr(),
                &mut connection,
                flags,
                core::ptr::null(),
            )
        };
        if err != libsqlite3_sys::SQLITE_OK {
            let message = sqlite_errmsg(connection, err);
            return Err((err, message));
        }

        cell.borrow_mut().push((cpath, connection));
        Ok(connection)
    })
}

fn sqlite_value_integer(value: i64) -> SqliteValue {
    SqliteValue {
        payload: SqliteValuePayload {
            integer: ManuallyDrop::new(value),
        },
        tag: SqliteValueTag::Integer,
    }
}

fn sqlite_value_real(value: f64) -> SqliteValue {
    SqliteValue {
        payload: SqliteValuePayload {
            real: ManuallyDrop::new(value),
        },
        tag: SqliteValueTag::Real,
    }
}

fn sqlite_value_string(value: RocStr) -> SqliteValue {
    SqliteValue {
        payload: SqliteValuePayload {
            string: ManuallyDrop::new(value),
        },
        tag: SqliteValueTag::String,
    }
}

fn sqlite_value_bytes(value: RocListWith<u8, false>) -> SqliteValue {
    SqliteValue {
        payload: SqliteValuePayload {
            bytes: ManuallyDrop::new(value),
        },
        tag: SqliteValueTag::Bytes,
    }
}

fn sqlite_value_null() -> SqliteValue {
    SqliteValue {
        payload: SqliteValuePayload { null: [] },
        tag: SqliteValueTag::Null,
    }
}

fn try_sqlite_prepare_ok(handle: *mut u64) -> HostSqlitePrepareResult {
    HostSqlitePrepareResult {
        payload: HostSqlitePrepareResultPayload {
            ok: ManuallyDrop::new(handle),
        },
        tag: HostSqlitePrepareResultTag::Ok,
    }
}

fn try_sqlite_prepare_err(error: SqliteError) -> HostSqlitePrepareResult {
    HostSqlitePrepareResult {
        payload: HostSqlitePrepareResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostSqlitePrepareResultTag::Err,
    }
}

fn try_sqlite_unit_ok() -> HostSqliteBindResult {
    HostSqliteBindResult {
        payload: HostSqliteBindResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: HostSqliteBindResultTag::Ok,
    }
}

fn try_sqlite_unit_err(error: SqliteError) -> HostSqliteBindResult {
    HostSqliteBindResult {
        payload: HostSqliteBindResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostSqliteBindResultTag::Err,
    }
}

fn try_sqlite_value_ok(value: SqliteValue) -> HostSqliteColumnValueResult {
    HostSqliteColumnValueResult {
        payload: HostSqliteColumnValueResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostSqliteColumnValueResultTag::Ok,
    }
}

fn try_sqlite_value_err(error: SqliteError) -> HostSqliteColumnValueResult {
    HostSqliteColumnValueResult {
        payload: HostSqliteColumnValueResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostSqliteColumnValueResultTag::Err,
    }
}

// `host_step!` marshals a Bool: true => a row is ready (SQLITE_ROW),
// false => the statement is done (SQLITE_DONE).
fn try_sqlite_step_ok(has_row: bool) -> HostSqliteStepResult {
    HostSqliteStepResult {
        payload: HostSqliteStepResultPayload {
            ok: ManuallyDrop::new(has_row),
        },
        tag: HostSqliteStepResultTag::Ok,
    }
}

fn try_sqlite_step_err(error: SqliteError) -> HostSqliteStepResult {
    HostSqliteStepResult {
        payload: HostSqliteStepResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostSqliteStepResultTag::Err,
    }
}

unsafe fn sqlite_bind_one(
    stmt: *mut libsqlite3_sys::sqlite3_stmt,
    index: c_int,
    value: &SqliteValue,
) -> c_int {
    match value.tag {
        SqliteValueTag::Integer => {
            libsqlite3_sys::sqlite3_bind_int64(stmt, index, *value.payload.integer)
        }
        SqliteValueTag::Real => {
            libsqlite3_sys::sqlite3_bind_double(stmt, index, *value.payload.real)
        }
        SqliteValueTag::String => {
            let text = value.payload.string.as_str();
            libsqlite3_sys::sqlite3_bind_text64(
                stmt,
                index,
                text.as_ptr() as *const c_char,
                text.len() as u64,
                sqlite_transient(),
                libsqlite3_sys::SQLITE_UTF8 as u8,
            )
        }
        SqliteValueTag::Bytes => {
            let bytes = value.payload.bytes.as_slice();
            libsqlite3_sys::sqlite3_bind_blob64(
                stmt,
                index,
                bytes.as_ptr() as *const c_void,
                bytes.len() as u64,
                sqlite_transient(),
            )
        }
        SqliteValueTag::Null => libsqlite3_sys::sqlite3_bind_null(stmt, index),
    }
}

fn sqlite_bind_all(
    stmt: &mut SqliteStatement,
    bindings: &[SqliteBindings],
    roc_host: &RocHost,
) -> HostSqliteBindResult {
    // Clear old bindings so callers must supply every parameter each time.
    let cleared = unsafe { libsqlite3_sys::sqlite3_clear_bindings(stmt.stmt) };
    if cleared != libsqlite3_sys::SQLITE_OK {
        return try_sqlite_unit_err(sqlite_err_from_stmt(stmt, cleared, roc_host));
    }

    for binding in bindings {
        let name = match CString::new(binding.name.as_str()) {
            Ok(name) => name,
            Err(_) => {
                return try_sqlite_unit_err(sqlite_error(
                    libsqlite3_sys::SQLITE_ERROR,
                    "binding name contained an interior nul byte",
                    roc_host,
                ));
            }
        };
        let index =
            unsafe { libsqlite3_sys::sqlite3_bind_parameter_index(stmt.stmt, name.as_ptr()) };
        if index == 0 {
            return try_sqlite_unit_err(sqlite_error(
                libsqlite3_sys::SQLITE_ERROR,
                &format!("unknown parameter: {}", binding.name.as_str()),
                roc_host,
            ));
        }
        let err = unsafe { sqlite_bind_one(stmt.stmt, index, &binding.value) };
        if err != libsqlite3_sys::SQLITE_OK {
            return try_sqlite_unit_err(sqlite_err_from_stmt(stmt, err, roc_host));
        }
    }

    try_sqlite_unit_ok()
}

#[no_mangle]
pub extern "C" fn hosted_sqlite_prepare(path: RocStr, query: RocStr) -> HostSqlitePrepareResult {
    let roc_host = roc_host();
    let path_string = path.as_str().to_owned();
    let query_string = query.as_str().to_owned();
    unsafe {
        path.decref(roc_host);
    }
    unsafe {
        query.decref(roc_host);
    }

    let connection = match sqlite_get_connection(&path_string) {
        Ok(connection) => connection,
        Err((code, message)) => {
            return try_sqlite_prepare_err(sqlite_error(code, &message, roc_host));
        }
    };

    let mut stmt: *mut libsqlite3_sys::sqlite3_stmt = core::ptr::null_mut();
    let err = unsafe {
        libsqlite3_sys::sqlite3_prepare_v2(
            connection,
            query_string.as_ptr() as *const c_char,
            query_string.len() as c_int,
            &mut stmt,
            core::ptr::null_mut(),
        )
    };
    if err != libsqlite3_sys::SQLITE_OK {
        let message = sqlite_errmsg(connection, err);
        return try_sqlite_prepare_err(sqlite_error(err, &message, roc_host));
    }

    let handle = box_sqlite_stmt(SqliteStatement { connection, stmt }, roc_host);
    try_sqlite_prepare_ok(handle)
}

#[no_mangle]
pub extern "C" fn hosted_sqlite_bind(
    handle: *mut u64,
    bindings: RocList<SqliteBindings>,
) -> HostSqliteBindResult {
    let roc_host = roc_host();
    let result = {
        let stmt = unsafe { sqlite_stmt_ref(handle) };
        sqlite_bind_all(stmt, bindings.as_slice(), roc_host)
    };
    for binding in bindings.as_slice() {
        decref_anon_struct55(*binding, roc_host);
    }
    unsafe {
        bindings.decref(roc_host);
    }
    release_sqlite_stmt(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_sqlite_columns(handle: *mut u64) -> RocList<RocStr> {
    let roc_host = roc_host();
    let stmt = unsafe { sqlite_stmt_ref(handle) };
    let count = unsafe { libsqlite3_sys::sqlite3_column_count(stmt.stmt) }.max(0) as usize;
    let list = unsafe { RocList::<RocStr>::allocate(count, roc_host) };
    for index in 0..count {
        let name = unsafe {
            let raw = libsqlite3_sys::sqlite3_column_name(stmt.stmt, index as c_int);
            if raw.is_null() {
                RocStr::from_str("", roc_host)
            } else {
                RocStr::from_str(CStr::from_ptr(raw).to_string_lossy().as_ref(), roc_host)
            }
        };
        unsafe {
            list.elements.add(index).write(name);
        }
    }
    release_sqlite_stmt(handle, roc_host);
    list
}

#[no_mangle]
pub extern "C" fn hosted_sqlite_column_value(
    handle: *mut u64,
    i: u64,
) -> HostSqliteColumnValueResult {
    let roc_host = roc_host();
    let result = {
        let stmt = unsafe { sqlite_stmt_ref(handle) };
        let count = unsafe { libsqlite3_sys::sqlite3_column_count(stmt.stmt) }.max(0) as u64;
        if i >= count {
            try_sqlite_value_err(sqlite_error(
                libsqlite3_sys::SQLITE_ERROR,
                &format!("column index out of range: {} of {}", i, count),
                roc_host,
            ))
        } else {
            let index = i as c_int;
            let value = unsafe {
                match libsqlite3_sys::sqlite3_column_type(stmt.stmt, index) {
                    libsqlite3_sys::SQLITE_INTEGER => {
                        sqlite_value_integer(libsqlite3_sys::sqlite3_column_int64(stmt.stmt, index))
                    }
                    libsqlite3_sys::SQLITE_FLOAT => {
                        sqlite_value_real(libsqlite3_sys::sqlite3_column_double(stmt.stmt, index))
                    }
                    libsqlite3_sys::SQLITE_TEXT => {
                        let text = libsqlite3_sys::sqlite3_column_text(stmt.stmt, index);
                        let len =
                            libsqlite3_sys::sqlite3_column_bytes(stmt.stmt, index).max(0) as usize;
                        let slice = if text.is_null() {
                            &[][..]
                        } else {
                            std::slice::from_raw_parts(text, len)
                        };
                        sqlite_value_string(RocStr::from_str(
                            String::from_utf8_lossy(slice).as_ref(),
                            roc_host,
                        ))
                    }
                    libsqlite3_sys::SQLITE_BLOB => {
                        let blob =
                            libsqlite3_sys::sqlite3_column_blob(stmt.stmt, index) as *const u8;
                        let len =
                            libsqlite3_sys::sqlite3_column_bytes(stmt.stmt, index).max(0) as usize;
                        let slice = if blob.is_null() {
                            &[][..]
                        } else {
                            std::slice::from_raw_parts(blob, len)
                        };
                        sqlite_value_bytes(roc_u8_list_from_slice(slice, roc_host))
                    }
                    _ => sqlite_value_null(),
                }
            };
            try_sqlite_value_ok(value)
        }
    };
    release_sqlite_stmt(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_sqlite_step(handle: *mut u64) -> HostSqliteStepResult {
    let roc_host = roc_host();
    let result = {
        let stmt = unsafe { sqlite_stmt_ref(handle) };
        let err = unsafe { libsqlite3_sys::sqlite3_step(stmt.stmt) };
        if err == libsqlite3_sys::SQLITE_ROW {
            try_sqlite_step_ok(true)
        } else if err == libsqlite3_sys::SQLITE_DONE {
            try_sqlite_step_ok(false)
        } else {
            try_sqlite_step_err(sqlite_err_from_stmt(stmt, err, roc_host))
        }
    };
    release_sqlite_stmt(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_sqlite_reset(handle: *mut u64) -> HostSqliteBindResult {
    let roc_host = roc_host();
    let result = {
        let stmt = unsafe { sqlite_stmt_ref(handle) };
        let err = unsafe { libsqlite3_sys::sqlite3_reset(stmt.stmt) };
        if err == libsqlite3_sys::SQLITE_OK {
            try_sqlite_unit_ok()
        } else {
            try_sqlite_unit_err(sqlite_err_from_stmt(stmt, err, roc_host))
        }
    };
    release_sqlite_stmt(handle, roc_host);
    result
}

// ============================================================================
// TCP
//
// `Tcp.Stream` (a `Box(U64)`) is represented by the generated glue as `*mut u64`:
// a boxed u64 holding a raw `*mut BufReader<TcpStream>`. The box is refcounted
// with `allocate_box`/`decref_box_with`; closing the socket happens in
// `drop_tcp_stream` when the last reference is released. Each host fn that takes
// a handle calls `release_tcp_stream` before returning to balance the incref Roc
// performs when the stream stays live.
//
// Errors cross the boundary as a `RocStr` carrying either "ErrorKind::<Variant>"
// (mapped back to a tag union in Tcp.roc) or "UnexpectedEof"; the Roc side parses
// them into `ConnectErr`/`StreamErr`.
// ----------------------------------------------------------------------------

const TCP_STREAM_BOX_ALIGN: usize = core::mem::align_of::<u64>();

fn box_tcp_stream(stream: BufReader<TcpStream>, roc_host: &RocHost) -> *mut u64 {
    let raw: *mut BufReader<TcpStream> = Box::into_raw(Box::new(stream));
    let boxed = unsafe {
        allocate_box(
            core::mem::size_of::<u64>(),
            TCP_STREAM_BOX_ALIGN,
            false,
            roc_host,
        )
    };
    unsafe {
        *(boxed as *mut u64) = raw as u64;
    }
    boxed as *mut u64
}

unsafe fn tcp_stream_ref<'a>(handle: *mut u64) -> &'a mut BufReader<TcpStream> {
    &mut *(*handle as *mut BufReader<TcpStream>)
}

extern "C" fn drop_tcp_stream(data_ptr: *mut c_void, _roc_host: *mut RocHost) {
    unsafe {
        let raw = *(data_ptr as *mut u64) as *mut BufReader<TcpStream>;
        if !raw.is_null() {
            drop(Box::from_raw(raw));
        }
    }
}

fn release_tcp_stream(handle: *mut u64, roc_host: &RocHost) {
    unsafe {
        decref_box_with(
            handle as RocBox,
            TCP_STREAM_BOX_ALIGN,
            false,
            Some(drop_tcp_stream),
            roc_host,
        );
    }
}

fn to_tcp_connect_err(err: io::Error, roc_host: &RocHost) -> RocStr {
    let message = match err.kind() {
        io::ErrorKind::PermissionDenied => "ErrorKind::PermissionDenied".to_string(),
        io::ErrorKind::AddrInUse => "ErrorKind::AddrInUse".to_string(),
        io::ErrorKind::AddrNotAvailable => "ErrorKind::AddrNotAvailable".to_string(),
        io::ErrorKind::ConnectionRefused => "ErrorKind::ConnectionRefused".to_string(),
        io::ErrorKind::Interrupted => "ErrorKind::Interrupted".to_string(),
        io::ErrorKind::TimedOut => "ErrorKind::TimedOut".to_string(),
        io::ErrorKind::Unsupported => "ErrorKind::Unsupported".to_string(),
        other => format!("{:?}", other),
    };
    RocStr::from_str(&message, roc_host)
}

fn to_tcp_stream_err(err: io::Error, roc_host: &RocHost) -> RocStr {
    let message = match err.kind() {
        io::ErrorKind::PermissionDenied => "ErrorKind::PermissionDenied".to_string(),
        io::ErrorKind::ConnectionRefused => "ErrorKind::ConnectionRefused".to_string(),
        io::ErrorKind::ConnectionReset => "ErrorKind::ConnectionReset".to_string(),
        io::ErrorKind::Interrupted => "ErrorKind::Interrupted".to_string(),
        io::ErrorKind::OutOfMemory => "ErrorKind::OutOfMemory".to_string(),
        io::ErrorKind::BrokenPipe => "ErrorKind::BrokenPipe".to_string(),
        other => format!("{:?}", other),
    };
    RocStr::from_str(&message, roc_host)
}

// `BufRead::read_until` ported from `roc_file::read_until`, accumulating into a
// plain Vec (the delimiter is included as the last byte when found).
fn tcp_read_until_impl(stream: &mut BufReader<TcpStream>, delim: u8) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();
    loop {
        let (done, used) = {
            let available = match stream.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            match available.iter().position(|&b| b == delim) {
                Some(i) => {
                    buffer.extend_from_slice(&available[..=i]);
                    (true, i + 1)
                }
                None => {
                    buffer.extend_from_slice(available);
                    (false, available.len())
                }
            }
        };
        stream.consume(used);
        if done || used == 0 {
            return Ok(buffer);
        }
    }
}

fn try_tcp_connect_ok(handle: *mut u64) -> HostTcpConnectResult {
    HostTcpConnectResult {
        payload: HostTcpConnectResultPayload {
            ok: ManuallyDrop::new(handle),
        },
        tag: HostTcpConnectResultTag::Ok,
    }
}

fn try_tcp_connect_err(error: RocStr) -> HostTcpConnectResult {
    HostTcpConnectResult {
        payload: HostTcpConnectResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostTcpConnectResultTag::Err,
    }
}

// The three read host fns share an identical result layout (`Try(List U8, Str)`).
fn try_tcp_read_ok(bytes: RocListWith<u8, false>) -> HostTcpReadUpToResult {
    HostTcpReadUpToResult {
        payload: HostTcpReadUpToResultPayload {
            ok: ManuallyDrop::new(bytes),
        },
        tag: HostTcpReadUpToResultTag::Ok,
    }
}

fn try_tcp_read_err(error: RocStr) -> HostTcpReadUpToResult {
    HostTcpReadUpToResult {
        payload: HostTcpReadUpToResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostTcpReadUpToResultTag::Err,
    }
}

fn try_tcp_write_ok() -> HostTcpWriteResult {
    HostTcpWriteResult {
        payload: HostTcpWriteResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: HostTcpWriteResultTag::Ok,
    }
}

fn try_tcp_write_err(error: RocStr) -> HostTcpWriteResult {
    HostTcpWriteResult {
        payload: HostTcpWriteResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostTcpWriteResultTag::Err,
    }
}

#[no_mangle]
pub extern "C" fn hosted_tcp_connect(host: RocStr, port: u16) -> HostTcpConnectResult {
    let roc_host = roc_host();
    let host_string = host.as_str().to_owned();
    unsafe {
        host.decref(roc_host);
    }

    match TcpStream::connect((host_string.as_str(), port)) {
        Ok(stream) => {
            let handle = box_tcp_stream(BufReader::new(stream), roc_host);
            try_tcp_connect_ok(handle)
        }
        Err(err) => try_tcp_connect_err(to_tcp_connect_err(err, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_tcp_read_up_to(
    handle: *mut u64,
    bytes_to_read: u64,
) -> HostTcpReadUpToResult {
    let roc_host = roc_host();
    let result = {
        let stream = unsafe { tcp_stream_ref(handle) };
        let mut chunk = stream.take(bytes_to_read);
        match chunk.fill_buf() {
            Ok(received) => {
                let received = received.to_vec();
                stream.consume(received.len());
                try_tcp_read_ok(roc_u8_list_from_slice(&received, roc_host))
            }
            Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
        }
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_read_exactly(
    handle: *mut u64,
    bytes_to_read: u64,
) -> HostTcpReadExactlyResult {
    let roc_host = roc_host();
    let result = {
        let stream = unsafe { tcp_stream_ref(handle) };
        let mut buffer = Vec::with_capacity(bytes_to_read as usize);
        let mut chunk = stream.take(bytes_to_read);
        match chunk.read_to_end(&mut buffer) {
            Ok(read) => {
                if (read as u64) < bytes_to_read {
                    try_tcp_read_err(RocStr::from_str("UnexpectedEof", roc_host))
                } else {
                    try_tcp_read_ok(roc_u8_list_from_slice(&buffer, roc_host))
                }
            }
            Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
        }
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_read_until(handle: *mut u64, byte: u8) -> HostTcpReadUntilResult {
    let roc_host = roc_host();
    let result = {
        let stream = unsafe { tcp_stream_ref(handle) };
        match tcp_read_until_impl(stream, byte) {
            Ok(buffer) => try_tcp_read_ok(roc_u8_list_from_slice(&buffer, roc_host)),
            Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
        }
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_write(
    handle: *mut u64,
    msg: RocListWith<u8, false>,
) -> HostTcpWriteResult {
    let roc_host = roc_host();
    let result = {
        let stream = unsafe { tcp_stream_ref(handle) };
        match stream.get_mut().write_all(msg.as_slice()) {
            Ok(()) => try_tcp_write_ok(),
            Err(err) => try_tcp_write_err(to_tcp_stream_err(err, roc_host)),
        }
    };
    unsafe {
        msg.decref(roc_host);
    }
    release_tcp_stream(handle, roc_host);
    result
}

// ============================================================================
// HTTP
//
// A single host effect, `hosted_http_send_request`, takes a fully-marshalled
// request record and returns a response record. Requests run on a thread-local
// current-thread tokio runtime driving a hyper client over a rustls (ring)
// TLS connector seeded with the system's native root certificates.
//
// Transport failures are surfaced to Roc as a typed `Try` error, separate from
// real HTTP responses. This avoids interpreting a legitimate server status/body
// pair as a platform transport failure.
// ----------------------------------------------------------------------------

// The generated glue names the request/response records by anonymous-struct
// number; alias them to stable semantic names. Host ABI headers are `(Str, Str)`
// tuples, rendered as a struct with `_0` (name) and `_1` (value) fields.
type HttpResponse = HostHttpSendRequestOk;
type HttpHeader = AnonStruct38;
type HttpResult = HostHttpSendRequestResult;
type HttpResultPayload = HostHttpSendRequestResultPayload;
type HttpResultTag = HostHttpSendRequestResultTag;
type HttpTransportErr = BadBodyOrNetworkErrorOrOtherOrTimeout;
type HttpTransportErrPayload = BadBodyOrNetworkErrorOrOtherOrTimeoutPayload;
type HttpTransportErrTag = BadBodyOrNetworkErrorOrOtherOrTimeoutTag;

thread_local! {
    static TOKIO_RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("failed to build tokio runtime");
}

// Numeric method tags must match `to_host_method` in platform/InternalHttp.roc.
fn as_hyper_method(method: u8, method_ext: &str) -> Option<hyper::Method> {
    match method {
        0 => Some(hyper::Method::CONNECT),
        1 => Some(hyper::Method::DELETE),
        2 => hyper::Method::from_bytes(method_ext.as_bytes()).ok(),
        3 => Some(hyper::Method::GET),
        4 => Some(hyper::Method::HEAD),
        5 => Some(hyper::Method::OPTIONS),
        6 => Some(hyper::Method::PATCH),
        7 => Some(hyper::Method::POST),
        8 => Some(hyper::Method::PUT),
        9 => Some(hyper::Method::TRACE),
        _ => None,
    }
}

fn http_ok(response: HttpResponse) -> HttpResult {
    HttpResult {
        payload: HttpResultPayload {
            ok: ManuallyDrop::new(response),
        },
        tag: HttpResultTag::Ok,
    }
}

fn http_err(error: HttpTransportErr) -> HttpResult {
    HttpResult {
        payload: HttpResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HttpResultTag::Err,
    }
}

fn http_err_timeout() -> HttpTransportErr {
    HttpTransportErr {
        payload: HttpTransportErrPayload { timeout: [] },
        tag: HttpTransportErrTag::Timeout,
    }
}

fn http_err_network_error() -> HttpTransportErr {
    HttpTransportErr {
        payload: HttpTransportErrPayload { network_error: [] },
        tag: HttpTransportErrTag::NetworkError,
    }
}

fn http_err_bad_body() -> HttpTransportErr {
    HttpTransportErr {
        payload: HttpTransportErrPayload { bad_body: [] },
        tag: HttpTransportErrTag::BadBody,
    }
}

fn http_err_other(message: &str, roc_host: &RocHost) -> HttpTransportErr {
    HttpTransportErr {
        payload: HttpTransportErrPayload {
            other: ManuallyDrop::new(roc_u8_list_from_slice(message.as_bytes(), roc_host)),
        },
        tag: HttpTransportErrTag::Other,
    }
}

fn build_hyper_request(
    args: &HostHttpSendRequestArgs,
) -> Result<hyper::Request<http_body_util::Full<bytes::Bytes>>, String> {
    let method = as_hyper_method(args.method, args.method_ext.as_str())
        .ok_or_else(|| "invalid HTTP method".to_string())?;
    let mut builder = hyper::Request::builder()
        .method(method)
        .uri(args.uri.as_str());

    // Default to text/plain unless the caller already set a Content-Type.
    let mut has_content_type = false;
    for header in args.headers.as_slice() {
        builder = builder.header(header._0.as_str(), header._1.as_str());
        if header._0.as_str().eq_ignore_ascii_case("Content-Type") {
            has_content_type = true;
        }
    }
    if !has_content_type {
        builder = builder.header("Content-Type", "text/plain");
    }

    let body = http_body_util::Full::new(bytes::Bytes::from(args.body.as_slice().to_vec()));
    builder.body(body).map_err(|err| err.to_string())
}

fn build_roc_headers(pairs: &[(String, String)], roc_host: &RocHost) -> RocList<HttpHeader> {
    let list = unsafe { RocList::<HttpHeader>::allocate(pairs.len(), roc_host) };
    for (index, (name, value)) in pairs.iter().enumerate() {
        let header = HttpHeader {
            _0: RocStr::from_str(name, roc_host),
            _1: RocStr::from_str(value, roc_host),
        };
        unsafe {
            list.elements.add(index).write(header);
        }
    }
    list
}

async fn async_send_request(
    request: hyper::Request<http_body_util::Full<bytes::Bytes>>,
    roc_host: &RocHost,
) -> HttpResult {
    use http_body_util::BodyExt;
    use hyper_rustls::HttpsConnectorBuilder;
    use hyper_util::client::legacy::Client;
    use hyper_util::rt::TokioExecutor;

    let https = match HttpsConnectorBuilder::new().with_native_roots() {
        Ok(builder) => builder.https_or_http().enable_http1().build(),
        Err(_) => return http_err(http_err_network_error()),
    };

    let client: Client<_, http_body_util::Full<bytes::Bytes>> =
        Client::builder(TokioExecutor::new()).build(https);

    match client.request(request).await {
        Ok(response) => {
            let status = response.status().as_u16();
            let pairs: Vec<(String, String)> = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();

            match response.into_body().collect().await {
                Ok(collected) => {
                    let bytes = collected.to_bytes();
                    http_ok(HttpResponse {
                        body: roc_u8_list_from_slice(&bytes, roc_host),
                        headers: build_roc_headers(&pairs, roc_host),
                        status,
                    })
                }
                Err(_) => http_err(http_err_bad_body()),
            }
        }
        Err(err) => {
            let detail = err.to_string();
            http_err(http_err_other(&detail, roc_host))
        }
    }
}

#[no_mangle]
pub extern "C" fn hosted_http_send_request(args: HostHttpSendRequestArgs) -> HttpResult {
    let roc_host = roc_host();
    let timeout_ms = args.timeout_ms;

    // Build the hyper request from the borrowed args, then release the owned
    // Roc values (the request has copied everything it needs).
    let request_result = build_hyper_request(&args);
    unsafe {
        args.body.decref(roc_host);
    }
    for header in args.headers.as_slice() {
        decref_anon_struct38(*header, roc_host);
    }
    unsafe {
        args.headers.decref(roc_host);
    }
    unsafe {
        args.method_ext.decref(roc_host);
    }
    unsafe {
        args.uri.decref(roc_host);
    }

    let request = match request_result {
        Ok(request) => request,
        Err(err) => return http_err(http_err_other(&err, roc_host)),
    };

    TOKIO_RUNTIME.with(|rt| {
        if timeout_ms > 0 {
            rt.block_on(async {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    async_send_request(request, roc_host),
                )
                .await
                {
                    Ok(response) => response,
                    Err(_) => http_err(http_err_timeout()),
                }
            })
        } else {
            rt.block_on(async_send_request(request, roc_host))
        }
    })
}

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
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    dir_io_err_from_io,
    dir_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    file_io_err_from_io,
    file_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    path_io_err_from_io,
    path_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    random_io_err_from_io,
    random_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    stderr_io_err_from_io,
    stderr_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    stdin_io_err_from_io,
    stdin_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);
define_common_io_err!(
    stdout_io_err_from_io,
    stdout_io_err_other,
    HostIOErr,
    HostIOErrTag,
    HostIOErrPayload
);

fn decref_roc_str_list(list: &RocList<RocStr>, roc_host: &RocHost) {
    for item in list.as_slice() {
        unsafe {
            item.decref(roc_host);
        }
    }
    unsafe {
        list.decref(roc_host);
    }
}

fn decref_host_cmd_arg(cmd: &Cmd, roc_host: &RocHost) {
    decref_roc_str_list(&cmd.args, roc_host);
    decref_roc_str_list(&cmd.envs, roc_host);
    unsafe {
        cmd.program.decref(roc_host);
    }
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

fn try_cmd_output_failure_err(error: HostIOErr) -> CmdOutputFailureResult {
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

fn try_dir_unit_err(error: HostIOErr) -> DirUnitResult {
    DirUnitResult {
        payload: DirUnitResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: DirUnitResultTag::Err,
    }
}

fn try_dir_list_ok(value: RocList<RocStr>) -> HostDirListResult {
    HostDirListResult {
        payload: HostDirListResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostDirListResultTag::Ok,
    }
}

fn try_dir_list_err(error: HostIOErr) -> HostDirListResult {
    HostDirListResult {
        payload: HostDirListResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostDirListResultTag::Err,
    }
}

fn try_env_str_ok(value: RocStr) -> HostEnvVarResult {
    HostEnvVarResult {
        payload: HostEnvVarResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostEnvVarResultTag::Ok,
    }
}

fn try_env_str_err(error: RocStr) -> HostEnvVarResult {
    HostEnvVarResult {
        payload: HostEnvVarResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostEnvVarResultTag::Err,
    }
}

fn try_env_cwd_ok(value: RocStr) -> HostEnvCwdResult {
    HostEnvCwdResult {
        payload: HostEnvCwdResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostEnvCwdResultTag::Ok,
    }
}

fn try_env_cwd_err() -> HostEnvCwdResult {
    HostEnvCwdResult {
        payload: HostEnvCwdResultPayload {
            err: ManuallyDrop::new(core::ptr::null_mut()),
        },
        tag: HostEnvCwdResultTag::Err,
    }
}

fn try_env_exe_path_ok(value: RocStr) -> HostEnvExePathResult {
    HostEnvExePathResult {
        payload: HostEnvExePathResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostEnvExePathResultTag::Ok,
    }
}

fn try_env_exe_path_err() -> HostEnvExePathResult {
    HostEnvExePathResult {
        payload: HostEnvExePathResultPayload {
            err: ManuallyDrop::new(core::ptr::null_mut()),
        },
        tag: HostEnvExePathResultTag::Err,
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

fn try_file_bytes_err(error: HostIOErr) -> FileBytesResult {
    FileBytesResult {
        payload: FileBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileBytesResultTag::Err,
    }
}

fn try_file_reader_ok(handle: *mut u64) -> FileReaderOpenResult {
    FileReaderOpenResult {
        payload: FileReaderOpenResultPayload {
            ok: ManuallyDrop::new(handle),
        },
        tag: FileReaderOpenResultTag::Ok,
    }
}

fn try_file_reader_err(error: HostIOErr) -> FileReaderOpenResult {
    FileReaderOpenResult {
        payload: FileReaderOpenResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileReaderOpenResultTag::Err,
    }
}

fn try_file_reader_line_ok(value: RocListWith<u8, false>) -> FileReaderLineResult {
    FileReaderLineResult {
        payload: FileReaderLineResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: FileReaderLineResultTag::Ok,
    }
}

fn try_file_reader_line_err(error: HostIOErr) -> FileReaderLineResult {
    FileReaderLineResult {
        payload: FileReaderLineResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileReaderLineResultTag::Err,
    }
}

fn try_file_write_bytes_ok() -> HostFileWriteBytesResult {
    HostFileWriteBytesResult {
        payload: HostFileWriteBytesResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: HostFileWriteBytesResultTag::Ok,
    }
}

fn try_file_write_bytes_err(error: HostIOErr) -> HostFileWriteBytesResult {
    HostFileWriteBytesResult {
        payload: HostFileWriteBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostFileWriteBytesResultTag::Err,
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

fn try_file_str_err(error: HostIOErr) -> FileStrResult {
    FileStrResult {
        payload: FileStrResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileStrResultTag::Err,
    }
}

fn try_file_write_utf8_ok() -> HostFileWriteUtf8Result {
    HostFileWriteUtf8Result {
        payload: HostFileWriteUtf8ResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: HostFileWriteUtf8ResultTag::Ok,
    }
}

fn try_file_write_utf8_err(error: HostIOErr) -> HostFileWriteUtf8Result {
    HostFileWriteUtf8Result {
        payload: HostFileWriteUtf8ResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostFileWriteUtf8ResultTag::Err,
    }
}

fn try_file_delete_ok() -> HostFileDeleteResult {
    HostFileDeleteResult {
        payload: HostFileDeleteResultPayload {
            ok: ManuallyDrop::new(()),
        },
        tag: HostFileDeleteResultTag::Ok,
    }
}

fn try_file_delete_err(error: HostIOErr) -> HostFileDeleteResult {
    HostFileDeleteResult {
        payload: HostFileDeleteResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostFileDeleteResultTag::Err,
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

fn try_file_size_err(error: HostIOErr) -> FileSizeResult {
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

fn try_file_bool_err(error: HostIOErr) -> FileBoolResult {
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

fn try_file_time_err(error: HostIOErr) -> FileTimeResult {
    FileTimeResult {
        payload: FileTimeResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: FileTimeResultTag::Err,
    }
}

fn try_locale_get_ok(value: RocStr) -> HostLocaleGetResult {
    HostLocaleGetResult {
        payload: HostLocaleGetResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostLocaleGetResultTag::Ok,
    }
}

fn try_path_type_ok(value: HostPathTypeOk) -> HostPathTypeResult {
    HostPathTypeResult {
        payload: HostPathTypeResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostPathTypeResultTag::Ok,
    }
}

fn try_path_type_err(error: HostIOErr) -> HostPathTypeResult {
    HostPathTypeResult {
        payload: HostPathTypeResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostPathTypeResultTag::Err,
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

fn try_random_u64_err(error: HostIOErr) -> RandomU64Result {
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

fn try_random_u32_err(error: HostIOErr) -> RandomU32Result {
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

fn try_stderr_unit_err(error: HostIOErr) -> StderrUnitResult {
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

fn try_stderr_bytes_err(error: HostIOErr) -> StderrBytesResult {
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

fn stdin_line_eof_or_err_io(error: HostIOErr) -> StdinLineReadErr {
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

fn stdin_bytes_eof_or_err_io(error: HostIOErr) -> StdinBytesReadErr {
    StdinBytesReadErr {
        payload: StdinBytesReadErrPayload {
            stdin_err: ManuallyDrop::new(error),
        },
        tag: StdinBytesReadErrTag::StdinErr,
    }
}

fn try_stdin_line_ok(value: RocStr) -> HostStdinLineResult {
    HostStdinLineResult {
        payload: HostStdinLineResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostStdinLineResultTag::Ok,
    }
}

fn try_stdin_line_err(error: StdinLineReadErr) -> HostStdinLineResult {
    HostStdinLineResult {
        payload: HostStdinLineResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostStdinLineResultTag::Err,
    }
}

fn try_stdin_bytes_ok(value: RocListWith<u8, false>) -> HostStdinBytesResult {
    HostStdinBytesResult {
        payload: HostStdinBytesResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostStdinBytesResultTag::Ok,
    }
}

fn try_stdin_bytes_err(error: StdinBytesReadErr) -> HostStdinBytesResult {
    HostStdinBytesResult {
        payload: HostStdinBytesResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostStdinBytesResultTag::Err,
    }
}

fn try_stdin_read_to_end_ok(value: RocListWith<u8, false>) -> HostStdinReadToEndResult {
    HostStdinReadToEndResult {
        payload: HostStdinReadToEndResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostStdinReadToEndResultTag::Ok,
    }
}

fn try_stdin_read_to_end_err(error: HostIOErr) -> HostStdinReadToEndResult {
    HostStdinReadToEndResult {
        payload: HostStdinReadToEndResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostStdinReadToEndResultTag::Err,
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

fn try_stdout_unit_err(error: HostIOErr) -> StdoutUnitResult {
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

fn try_stdout_bytes_err(error: HostIOErr) -> StdoutBytesResult {
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
            let stdout_bytes = roc_u8_list_from_slice(&output.stdout, roc_host);
            let stderr_bytes = roc_u8_list_from_slice(&output.stderr, roc_host);

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
                    unsafe {
                        stdout_bytes.decref(roc_host);
                    }
                    unsafe {
                        stderr_bytes.decref(roc_host);
                    }
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
    unsafe {
        path.decref(roc_host);
    }
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
pub extern "C" fn hosted_dir_list(path: RocStr) -> HostDirListResult {
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
            let list = unsafe { RocList::<RocStr>::allocate(entries.len(), roc_host) };
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
pub extern "C" fn hosted_env_cwd() -> HostEnvCwdResult {
    let roc_host = roc_host();
    match std::env::current_dir() {
        Ok(path) => try_env_cwd_ok(RocStr::from_str(path.to_string_lossy().as_ref(), roc_host)),
        Err(_) => try_env_cwd_err(),
    }
}

#[no_mangle]
pub extern "C" fn hosted_env_exe_path() -> HostEnvExePathResult {
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
pub extern "C" fn hosted_env_var(name: RocStr) -> HostEnvVarResult {
    let roc_host = roc_host();
    let key = name.as_str().to_owned();
    match std::env::var_os(&key) {
        Some(value) => {
            unsafe {
                name.decref(roc_host);
            }
            try_env_str_ok(RocStr::from_str(value.to_string_lossy().as_ref(), roc_host))
        }
        None => try_env_str_err(name),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_delete(path: RocStr) -> HostFileDeleteResult {
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
        Ok(bytes) => try_file_bytes_ok(roc_u8_list_from_slice(&bytes, roc_host)),
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

// ============================================================================
// Buffered file readers
//
// `File.Reader` (a `Box(U64)`) is represented by the generated glue as `*mut u64`:
// a boxed u64 holding a raw `*mut BufReader<fs::File>`. The box is refcounted
// with `allocate_box`/`decref_box_with`; closing the file happens in
// `drop_file_reader` when the last reference is released.
// ----------------------------------------------------------------------------

const FILE_READER_BOX_ALIGN: usize = core::mem::align_of::<u64>();

fn box_file_reader(reader: BufReader<fs::File>, roc_host: &RocHost) -> *mut u64 {
    let raw: *mut BufReader<fs::File> = Box::into_raw(Box::new(reader));
    let boxed = unsafe {
        allocate_box(
            core::mem::size_of::<u64>(),
            FILE_READER_BOX_ALIGN,
            false,
            roc_host,
        )
    };
    unsafe {
        *(boxed as *mut u64) = raw as u64;
    }
    boxed as *mut u64
}

unsafe fn file_reader_ref<'a>(handle: *mut u64) -> &'a mut BufReader<fs::File> {
    &mut *(*handle as *mut BufReader<fs::File>)
}

extern "C" fn drop_file_reader(data_ptr: *mut c_void, _roc_host: *mut RocHost) {
    unsafe {
        let raw = *(data_ptr as *mut u64) as *mut BufReader<fs::File>;
        if !raw.is_null() {
            drop(Box::from_raw(raw));
        }
    }
}

fn release_file_reader(handle: *mut u64, roc_host: &RocHost) {
    unsafe {
        decref_box_with(
            handle as RocBox,
            FILE_READER_BOX_ALIGN,
            false,
            Some(drop_file_reader),
            roc_host,
        );
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_open_reader(path: RocStr, capacity: u64) -> FileReaderOpenResult {
    let roc_host = roc_host();
    match fs::File::open(path_from_roc_str(path, roc_host)) {
        Ok(file) => {
            let reader = if capacity == 0 {
                BufReader::new(file)
            } else {
                BufReader::with_capacity(capacity as usize, file)
            };
            try_file_reader_ok(box_file_reader(reader, roc_host))
        }
        Err(error) => try_file_reader_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_read_line(handle: *mut u64) -> FileReaderLineResult {
    let roc_host = roc_host();
    let result = {
        let reader = unsafe { file_reader_ref(handle) };
        let mut buffer = Vec::new();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(_) => try_file_reader_line_ok(roc_u8_list_from_slice(&buffer, roc_host)),
            Err(error) => try_file_reader_line_err(file_io_err_from_io(&error, roc_host)),
        }
    };
    release_file_reader(handle, roc_host);
    result
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
) -> HostFileWriteBytesResult {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    let result = fs::write(path_string, bytes.as_slice());
    unsafe {
        bytes.decref(roc_host);
    }

    match result {
        Ok(()) => try_file_write_bytes_ok(),
        Err(error) => try_file_write_bytes_err(file_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_file_write_utf8(path: RocStr, content: RocStr) -> HostFileWriteUtf8Result {
    let roc_host = roc_host();
    let path_string = path_from_roc_str(path, roc_host);
    let content_string = content.as_str().to_owned();
    unsafe {
        content.decref(roc_host);
    }

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
    let list = unsafe { RocList::<RocStr>::allocate(locales.len(), roc_host) };

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
pub extern "C" fn hosted_locale_get() -> HostLocaleGetResult {
    let roc_host = roc_host();
    try_locale_get_ok(RocStr::from_str(&locale_get_string(), roc_host))
}

fn path_buf_from_bytes(path: RocListWith<u8, false>, roc_host: &RocHost) -> std::path::PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let path_buf = std::path::PathBuf::from(std::ffi::OsStr::from_bytes(path.as_slice()));
        unsafe {
            path.decref(roc_host);
        }
        path_buf
    }

    #[cfg(not(unix))]
    {
        let path_buf = std::path::PathBuf::from(String::from_utf8_lossy(path.as_slice()).as_ref());
        unsafe {
            path.decref(roc_host);
        }
        path_buf
    }
}

#[no_mangle]
pub extern "C" fn hosted_path_type(path: RocListWith<u8, false>) -> HostPathTypeResult {
    let roc_host = roc_host();
    let path = path_buf_from_bytes(path, roc_host);

    match path.symlink_metadata() {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            try_path_type_ok(HostPathTypeOk {
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
    unsafe {
        message.decref(roc_host);
    }

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
    unsafe {
        message.decref(roc_host);
    }

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
    unsafe {
        bytes.decref(roc_host);
    }

    match result {
        Ok(()) => try_stderr_bytes_ok(),
        Err(error) => try_stderr_bytes_err(stderr_io_err_from_io(&error, roc_host)),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdin_line() -> HostStdinLineResult {
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
pub extern "C" fn hosted_stdin_bytes() -> HostStdinBytesResult {
    let roc_host = roc_host();
    let mut buffer = [0u8; 16_384];
    match io::stdin().lock().read(&mut buffer) {
        Ok(0) => try_stdin_bytes_err(stdin_bytes_eof_or_err_eof()),
        Ok(bytes_read) => {
            try_stdin_bytes_ok(roc_u8_list_from_slice(&buffer[..bytes_read], roc_host))
        }
        Err(error) => try_stdin_bytes_err(stdin_bytes_eof_or_err_io(stdin_io_err_from_io(
            &error, roc_host,
        ))),
    }
}

#[no_mangle]
pub extern "C" fn hosted_stdin_read_to_end() -> HostStdinReadToEndResult {
    let roc_host = roc_host();
    let mut buffer = Vec::new();
    match io::stdin().lock().read_to_end(&mut buffer) {
        Ok(_) => try_stdin_read_to_end_ok(roc_u8_list_from_slice(&buffer, roc_host)),
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
    unsafe {
        message.decref(roc_host);
    }

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
    unsafe {
        message.decref(roc_host);
    }

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
    unsafe {
        bytes.decref(roc_host);
    }

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

    let list = unsafe { RocList::<RocStr>::allocate(argc as usize, roc_host) };
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

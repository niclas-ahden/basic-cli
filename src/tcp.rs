use core::mem::ManuallyDrop;
use std::ffi::c_void;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use crate::roc_platform_abi::*;
use crate::{roc_host, roc_u8_list_from_slice};

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

const TCP_STREAM_BOX_ALIGN: usize = core::mem::align_of::<u64>();

fn box_tcp_stream(stream: BufReader<TcpStream>, roc_host: &RocHost) -> *mut u64 {
    let raw: *mut BufReader<TcpStream> = Box::into_raw(Box::new(stream));
    let boxed = allocate_box(
        core::mem::size_of::<u64>(),
        TCP_STREAM_BOX_ALIGN,
        false,
        roc_host,
    );
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
    decref_box_with(
        handle as RocBox,
        TCP_STREAM_BOX_ALIGN,
        false,
        Some(drop_tcp_stream),
        roc_host,
    );
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
    host.decref(roc_host);

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
    msg.decref(roc_host);
    release_tcp_stream(handle, roc_host);
    result
}

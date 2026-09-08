use core::mem::ManuallyDrop;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{mpsc, Arc, Condvar, Mutex, Weak};
use std::thread;
use std::time::{Duration, Instant};

use crate::roc_platform_abi::*;
use crate::{roc_host, roc_u8_list_from_slice};

// Tcp streams use the shared resource allocator so final release in either
// compiled Roc or a hosted effect closes the native buffered socket.
// Each effect balances its transferred handle reference before returning.
//
// The stream resource is a `StreamCell`: the buffered socket plus a weak link
// to the pool it was acquired from, if any. Dropping a cell that still owns
// its socket gives the pool its connection slot back, so a checked-out stream
// that is never released cannot wedge the pool. `hosted_tcp_pool_release`
// instead moves the socket out of the cell into the pool's idle set, after
// which any later use of that handle fails with `StreamNotFound`.
//
// A `Host.TcpPool` is a resource holding an `Arc<PoolEntry>`. Pools are
// host-managed: a bound (`max_connections`) on total connections (checked out
// plus idle), a Condvar to wait on when at the cap, and an idle set carrying a
// caller-owned metadata blob per connection, which protocol libraries use to
// persist per-connection session state across checkouts.
//
// Errors cross the boundary as a `RocStr` carrying either "ErrorKind::<Variant>"
// (mapped back to a tag union in Tcp.roc), "StreamNotFound", "UnexpectedEof",
// or "LimitExceeded". The Roc side parses them into the public TCP error tags.

/// How long `pool_acquire` waits for a connection before failing with
/// `ErrorKind::TimedOut`. The budget covers both waiting for a release when
/// the pool is at `max_connections` and dialing a new connection.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);

/// Idle connections older than this are closed instead of reused (the
/// server side has likely timed them out anyway).
const IDLE_TTL: Duration = Duration::from_secs(600);

/// What a `Tcp.Stream` resource holds.
struct StreamCell {
    /// None once `hosted_tcp_pool_release` has moved the socket into the pool.
    reader: Option<BufReader<TcpStream>>,
    /// The pool this stream was acquired from, if any.
    pool: Option<Weak<PoolEntry>>,
}

impl Drop for StreamCell {
    fn drop(&mut self) {
        // A cell that still owns its socket is a checked-out connection going
        // away, so closing it frees its slot. A cell emptied by pool_release
        // handed its connection to the idle set, which keeps the slot.
        if let Some(reader) = self.reader.take() {
            drop(reader);
            if let Some(pool) = self.pool.as_ref().and_then(Weak::upgrade) {
                pool.release_slot();
            }
        }
    }
}

struct IdleConn {
    reader: BufReader<TcpStream>,
    metadata: Vec<u8>,
    idled_at: Instant,
}

struct PoolInner {
    host: String,
    port: u16,
    /// Bound on TOTAL connections attributed to the pool: checked-out + idle.
    /// When the pool is at this cap, `pool_acquire` waits for a release
    /// instead of dialing.
    max_connections: usize,
    idle: Vec<IdleConn>,
    /// Checked-out + idle connections currently attributed to this pool.
    total: usize,
}

struct PoolEntry {
    inner: Mutex<PoolInner>,
    /// Signalled whenever capacity frees up (release, drop of a checked-out
    /// stream, or a connection going idle).
    released: Condvar,
}

impl PoolEntry {
    /// Give a connection slot back to the pool and wake one waiter.
    fn release_slot(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.total = inner.total.saturating_sub(1);
        drop(inner);
        self.released.notify_one();
    }
}

pub(crate) fn box_tcp_stream(stream: BufReader<TcpStream>, roc_host: &RocHost) -> *mut u64 {
    box_stream_cell(
        StreamCell {
            reader: Some(stream),
            pool: None,
        },
        roc_host,
    )
}

fn box_stream_cell(cell: StreamCell, roc_host: &RocHost) -> *mut u64 {
    crate::resources::box_resource(cell, roc_host)
}

/// The socket behind a stream handle, or None when `hosted_tcp_pool_release`
/// already moved it into its pool (the handle outlived its release).
unsafe fn tcp_stream_ref<'a>(handle: *mut u64) -> Option<&'a mut BufReader<TcpStream>> {
    unsafe { crate::resources::resource_ref::<StreamCell>(handle) }
        .reader
        .as_mut()
}

fn release_tcp_stream(handle: *mut u64, roc_host: &RocHost) {
    crate::resources::release(handle, roc_host);
}

fn release_tcp_pool(handle: *mut u64, roc_host: &RocHost) {
    crate::resources::release(handle, roc_host);
}

fn stream_not_found(roc_host: &RocHost) -> RocStr {
    RocStr::from_str("StreamNotFound", roc_host)
}

pub(crate) fn to_tcp_connect_err(err: io::Error, roc_host: &RocHost) -> RocStr {
    let message = match err.kind() {
        io::ErrorKind::PermissionDenied => "ErrorKind::PermissionDenied".to_string(),
        io::ErrorKind::AddrInUse => "ErrorKind::AddrInUse".to_string(),
        io::ErrorKind::AddrNotAvailable => "ErrorKind::AddrNotAvailable".to_string(),
        io::ErrorKind::ConnectionRefused => "ErrorKind::ConnectionRefused".to_string(),
        io::ErrorKind::Interrupted => "ErrorKind::Interrupted".to_string(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "ErrorKind::TimedOut".to_string(),
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
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "ErrorKind::TimedOut".to_string(),
        io::ErrorKind::OutOfMemory => "ErrorKind::OutOfMemory".to_string(),
        io::ErrorKind::BrokenPipe => "ErrorKind::BrokenPipe".to_string(),
        other => format!("{:?}", other),
    };
    RocStr::from_str(&message, roc_host)
}

fn timed_out() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "TCP operation timed out")
}

pub(crate) fn deadline_from_timeout(timeout_ms: u64) -> io::Result<Instant> {
    if timeout_ms == 0 {
        return Err(timed_out());
    }

    Instant::now()
        .checked_add(Duration::from_millis(timeout_ms))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "TCP timeout is too large"))
}

pub(crate) fn remaining_time(deadline: Instant) -> io::Result<Duration> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(timed_out)?;
    if remaining.is_zero() {
        Err(timed_out())
    } else {
        Ok(remaining)
    }
}

pub(crate) fn resolve_with_deadline(
    host: String,
    port: u16,
    deadline: Instant,
) -> io::Result<Vec<SocketAddr>> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(vec![SocketAddr::new(ip, port)]);
    }

    let (sender, receiver) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("basic-cli-tcp-resolve".to_string())
        .spawn(move || {
            let resolved = (host.as_str(), port)
                .to_socket_addrs()
                .map(|addresses| addresses.collect());
            let _ = sender.send(resolved);
        })?;

    match receiver.recv_timeout(remaining_time(deadline)?) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(timed_out()),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(io::Error::new(
            io::ErrorKind::Other,
            "TCP address resolution worker stopped unexpectedly",
        )),
    }
}

fn tcp_connect_with_timeout_impl(
    host: String,
    port: u16,
    timeout_ms: u64,
) -> io::Result<TcpStream> {
    let deadline = deadline_from_timeout(timeout_ms)?;
    let addresses = resolve_with_deadline(host, port, deadline)?;
    let mut last_error = None;

    for address in addresses {
        match TcpStream::connect_timeout(&address, remaining_time(deadline)?) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "hostname resolved to no TCP addresses",
        )
    }))
}

fn set_read_deadline(stream: &TcpStream, deadline: Option<Instant>) -> io::Result<()> {
    if let Some(deadline) = deadline {
        stream.set_read_timeout(Some(remaining_time(deadline)?))?;
    }
    Ok(())
}

fn set_write_deadline(stream: &TcpStream, deadline: Option<Instant>) -> io::Result<()> {
    if let Some(deadline) = deadline {
        stream.set_write_timeout(Some(remaining_time(deadline)?))?;
    }
    Ok(())
}

fn restore_timeout<T>(result: io::Result<T>, restored: io::Result<()>) -> io::Result<T> {
    match result {
        Err(error) => Err(error),
        Ok(value) => restored.map(|()| value),
    }
}

fn with_read_timeout<T>(
    stream: &mut BufReader<TcpStream>,
    timeout_ms: u64,
    operation: impl FnOnce(&mut BufReader<TcpStream>, Instant) -> io::Result<T>,
) -> io::Result<T> {
    let deadline = deadline_from_timeout(timeout_ms)?;
    let previous_timeout = stream.get_ref().read_timeout()?;
    let result = operation(stream, deadline);
    let restored = stream.get_ref().set_read_timeout(previous_timeout);
    restore_timeout(result, restored)
}

fn tcp_read_up_to_impl(
    stream: &mut BufReader<TcpStream>,
    bytes_to_read: u64,
    deadline: Option<Instant>,
) -> io::Result<Vec<u8>> {
    if bytes_to_read == 0 {
        return Ok(Vec::new());
    }

    set_read_deadline(stream.get_ref(), deadline)?;
    let available = stream.fill_buf()?;
    let limit = usize::try_from(bytes_to_read).unwrap_or(usize::MAX);
    let used = available.len().min(limit);
    let received = available[..used].to_vec();
    stream.consume(used);
    Ok(received)
}

fn tcp_read_exactly_impl(
    stream: &mut BufReader<TcpStream>,
    bytes_to_read: u64,
    deadline: Option<Instant>,
) -> io::Result<Vec<u8>> {
    let capacity = usize::try_from(bytes_to_read).map_err(|_| {
        io::Error::new(
            io::ErrorKind::OutOfMemory,
            "requested TCP read does not fit in memory",
        )
    })?;
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(capacity).map_err(|_| {
        io::Error::new(
            io::ErrorKind::OutOfMemory,
            "could not reserve memory for TCP read",
        )
    })?;
    let mut chunk = [0; 8192];

    while buffer.len() < capacity {
        set_read_deadline(stream.get_ref(), deadline)?;
        let remaining = capacity - buffer.len();
        let chunk_len = remaining.min(chunk.len());
        match stream.read(&mut chunk[..chunk_len]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "TCP stream ended before the requested bytes arrived",
                ))
            }
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(ref error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }

    Ok(buffer)
}

enum ReadUntilError {
    Io(io::Error),
    LimitExceeded,
}

impl From<io::Error> for ReadUntilError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

// `BufRead::read_until` ported from `roc_file::read_until`, with an optional
// caller-selected maximum. The delimiter is included as the last byte when found.
fn tcp_read_until_impl(
    stream: &mut BufReader<TcpStream>,
    delim: u8,
    max_bytes: Option<u64>,
    deadline: Option<Instant>,
) -> Result<Vec<u8>, ReadUntilError> {
    let mut buffer = Vec::new();
    loop {
        if max_bytes.is_some_and(|limit| buffer.len() as u64 >= limit) {
            return Err(ReadUntilError::LimitExceeded);
        }

        set_read_deadline(stream.get_ref(), deadline)?;
        let (done, used, limit_exceeded) = {
            let available = match stream.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(ReadUntilError::Io(e)),
            };
            let allowed = max_bytes
                .map(|limit| usize::try_from(limit - buffer.len() as u64).unwrap_or(usize::MAX))
                .unwrap_or(available.len())
                .min(available.len());
            match available[..allowed].iter().position(|&b| b == delim) {
                Some(i) => {
                    buffer.extend_from_slice(&available[..=i]);
                    (true, i + 1, false)
                }
                None => {
                    buffer.extend_from_slice(&available[..allowed]);
                    (false, allowed, allowed < available.len())
                }
            }
        };
        stream.consume(used);
        if limit_exceeded {
            return Err(ReadUntilError::LimitExceeded);
        } else if done || used == 0 {
            return Ok(buffer);
        }
    }
}

fn tcp_write_impl(
    stream: &mut TcpStream,
    bytes: &[u8],
    deadline: Option<Instant>,
) -> io::Result<()> {
    let mut written = 0;
    while written < bytes.len() {
        set_write_deadline(stream, deadline)?;
        match stream.write(&bytes[written..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
            Ok(count) => written += count,
            Err(ref error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn with_write_timeout<T>(
    stream: &mut TcpStream,
    timeout_ms: u64,
    operation: impl FnOnce(&mut TcpStream, Instant) -> io::Result<T>,
) -> io::Result<T> {
    let deadline = deadline_from_timeout(timeout_ms)?;
    let previous_timeout = stream.write_timeout()?;
    let result = operation(stream, deadline);
    let restored = stream.set_write_timeout(previous_timeout);
    restore_timeout(result, restored)
}

fn with_read_until_timeout(
    stream: &mut BufReader<TcpStream>,
    delim: u8,
    max_bytes: Option<u64>,
    timeout_ms: u64,
) -> Result<Vec<u8>, ReadUntilError> {
    let deadline = deadline_from_timeout(timeout_ms)?;
    let previous_timeout = stream.get_ref().read_timeout()?;
    let result = tcp_read_until_impl(stream, delim, max_bytes, Some(deadline));
    let restored = stream.get_ref().set_read_timeout(previous_timeout);
    match result {
        Err(error) => Err(error),
        Ok(value) => restored.map(|()| value).map_err(ReadUntilError::Io),
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
        payload: HostTcpWriteResultPayload { ok: [] },
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
pub extern "C" fn hosted_tcp_connect(
    host: RocStr,
    port: u16,
    timeout_ms: u64,
) -> HostTcpConnectResult {
    let roc_host = roc_host();
    let host_string = host.as_str().to_owned();
    unsafe { host.decref(roc_host) };

    match tcp_connect_with_timeout_impl(host_string, port, timeout_ms) {
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
    timeout_ms: u64,
) -> HostTcpReadUpToResult {
    let roc_host = roc_host();
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_read_err(stream_not_found(roc_host)),
        Some(stream) => match with_read_timeout(stream, timeout_ms, |stream, deadline| {
            tcp_read_up_to_impl(stream, bytes_to_read, Some(deadline))
        }) {
            Ok(received) => try_tcp_read_ok(roc_u8_list_from_slice(&received, roc_host)),
            Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
        },
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_read_exactly(
    handle: *mut u64,
    bytes_to_read: u64,
    timeout_ms: u64,
) -> HostTcpReadExactlyResult {
    let roc_host = roc_host();
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_read_err(stream_not_found(roc_host)),
        Some(stream) => match with_read_timeout(stream, timeout_ms, |stream, deadline| {
            tcp_read_exactly_impl(stream, bytes_to_read, Some(deadline))
        }) {
            Ok(buffer) => try_tcp_read_ok(roc_u8_list_from_slice(&buffer, roc_host)),
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                try_tcp_read_err(RocStr::from_str("UnexpectedEof", roc_host))
            }
            Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
        },
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_read_until(
    handle: *mut u64,
    byte: u8,
    max_bytes: u64,
    timeout_ms: u64,
) -> HostTcpReadUntilResult {
    let roc_host = roc_host();
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_read_err(stream_not_found(roc_host)),
        Some(stream) => match with_read_until_timeout(stream, byte, Some(max_bytes), timeout_ms) {
            Ok(buffer) => try_tcp_read_ok(roc_u8_list_from_slice(&buffer, roc_host)),
            Err(ReadUntilError::Io(err)) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
            Err(ReadUntilError::LimitExceeded) => {
                try_tcp_read_err(RocStr::from_str("LimitExceeded", roc_host))
            }
        },
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_write(
    handle: *mut u64,
    msg: RocListWith<u8, false>,
    timeout_ms: u64,
) -> HostTcpWriteResult {
    let roc_host = roc_host();
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_write_err(stream_not_found(roc_host)),
        Some(stream) => match with_write_timeout(stream.get_mut(), timeout_ms, |stream, deadline| {
            tcp_write_impl(stream, msg.as_slice(), Some(deadline))
        }) {
            Ok(()) => try_tcp_write_ok(),
            Err(err) => try_tcp_write_err(to_tcp_stream_err(err, roc_host)),
        },
    };
    unsafe { msg.decref(roc_host) };
    release_tcp_stream(handle, roc_host);
    result
}

/// Shut the socket down in both directions immediately. Resources (and the
/// pool slot, for pooled streams) are still freed by the resource finalizer
/// when the last reference drops.
#[no_mangle]
pub extern "C" fn hosted_tcp_shutdown(handle: *mut u64) {
    let roc_host = roc_host();
    if let Some(stream) = unsafe { tcp_stream_ref(handle) } {
        let _ = stream.get_ref().shutdown(Shutdown::Both);
    }
    release_tcp_stream(handle, roc_host);
}

// ============================================================================
// Connection pools
// ============================================================================

/// Convert what is left of a deadline back into the millisecond timeout the
/// connect path takes. A sub-millisecond remainder becomes 1ms rather than 0,
/// which `deadline_from_timeout` reads as "fail immediately".
fn timeout_ms_from(remaining: Duration) -> u64 {
    u64::try_from(remaining.as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

#[no_mangle]
pub extern "C" fn hosted_tcp_pool_create(host: RocStr, port: u16, max_connections: u64) -> *mut u64 {
    let roc_host = roc_host();
    let host_string = host.as_str().to_owned();
    unsafe { host.decref(roc_host) };

    let pool = Arc::new(PoolEntry {
        inner: Mutex::new(PoolInner {
            host: host_string,
            port,
            max_connections: usize::try_from(max_connections).unwrap_or(usize::MAX).max(1),
            idle: Vec::new(),
            total: 0,
        }),
        released: Condvar::new(),
    });
    crate::resources::box_resource(pool, roc_host)
}

/// True if a pooled idle connection is still usable: readable with no
/// buffered protocol bytes means the peer closed or desynced, while
/// `WouldBlock` means healthy and idle.
fn idle_conn_is_live(conn: &IdleConn) -> bool {
    if !conn.reader.buffer().is_empty() {
        // Data the last user never consumed: protocol desync, don't reuse.
        return false;
    }
    let stream = conn.reader.get_ref();
    if stream.set_nonblocking(true).is_err() {
        return false;
    }
    let mut probe = [0u8; 1];
    let live = match stream.peek(&mut probe) {
        Ok(0) => false,                                          // peer closed
        Ok(_) => false,                                          // unexpected bytes: desync
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => true, // idle and open
        Err(_) => false,
    };
    if stream.set_nonblocking(false).is_err() {
        return false;
    }
    live
}

fn try_pool_acquire_ok(value: HostTcpPoolAcquireOk) -> HostTcpPoolAcquireResult {
    HostTcpPoolAcquireResult {
        payload: HostTcpPoolAcquireResultPayload {
            ok: ManuallyDrop::new(value),
        },
        tag: HostTcpPoolAcquireResultTag::Ok,
    }
}

fn try_pool_acquire_err(error: RocStr) -> HostTcpPoolAcquireResult {
    HostTcpPoolAcquireResult {
        payload: HostTcpPoolAcquireResultPayload {
            err: ManuallyDrop::new(error),
        },
        tag: HostTcpPoolAcquireResultTag::Err,
    }
}

#[no_mangle]
pub extern "C" fn hosted_tcp_pool_acquire(pool_handle: *mut u64) -> HostTcpPoolAcquireResult {
    let roc_host = roc_host();
    // Clone the Arc before balancing the handle reference. On the Roc `Pool`
    // value's last use, Roc hands ownership over without an incref, so the
    // release below finalizes the resource, and the clone is what keeps the
    // pool alive for the rest of this acquire.
    let pool = unsafe { crate::resources::resource_ref::<Arc<PoolEntry>>(pool_handle) }.clone();
    release_tcp_pool(pool_handle, roc_host);

    let deadline = Instant::now() + ACQUIRE_TIMEOUT;
    let mut inner = pool.inner.lock().unwrap_or_else(|p| p.into_inner());
    loop {
        // Recycled connection if a live one is idle (dead and stale ones free
        // their slot).
        while let Some(conn) = inner.idle.pop() {
            if conn.idled_at.elapsed() > IDLE_TTL || !idle_conn_is_live(&conn) {
                inner.total = inner.total.saturating_sub(1);
                continue;
            }
            drop(inner);
            let IdleConn {
                reader, metadata, ..
            } = conn;
            let handle = box_stream_cell(
                StreamCell {
                    reader: Some(reader),
                    pool: Some(Arc::downgrade(&pool)),
                },
                roc_host,
            );
            return try_pool_acquire_ok(HostTcpPoolAcquireOk {
                metadata: roc_u8_list_from_slice(&metadata, roc_host),
                stream: handle,
                fresh: false,
            });
        }

        // Nothing idle: dial if below the cap (reserving the slot first so
        // no lock is held during connect). The dial gets whatever is left of
        // the acquire budget, so ACQUIRE_TIMEOUT bounds the whole call rather
        // than only the wait for a release.
        if inner.total < inner.max_connections {
            inner.total += 1;
            let host = inner.host.clone();
            let port = inner.port;
            drop(inner);
            let dialed = remaining_time(deadline).and_then(|remaining| {
                tcp_connect_with_timeout_impl(host, port, timeout_ms_from(remaining))
            });
            match dialed {
                Ok(stream) => {
                    let handle = box_stream_cell(
                        StreamCell {
                            reader: Some(BufReader::new(stream)),
                            pool: Some(Arc::downgrade(&pool)),
                        },
                        roc_host,
                    );
                    return try_pool_acquire_ok(HostTcpPoolAcquireOk {
                        metadata: roc_u8_list_from_slice(&[], roc_host),
                        stream: handle,
                        fresh: true,
                    });
                }
                Err(e) => {
                    pool.release_slot(); // give the reserved slot back
                    return try_pool_acquire_err(to_tcp_connect_err(e, roc_host));
                }
            }
        }

        // At max_connections: wait for a release (or time out).
        let now = Instant::now();
        if now >= deadline {
            return try_pool_acquire_err(RocStr::from_str("ErrorKind::TimedOut", roc_host));
        }
        let (guard, _timed_out) = pool
            .released
            .wait_timeout(inner, deadline - now)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner = guard;
    }
}

#[no_mangle]
pub extern "C" fn hosted_tcp_pool_release(
    handle: *mut u64,
    reusable: bool,
    metadata: RocListWith<u8, false>,
) {
    let roc_host = roc_host();
    let metadata_vec = metadata.as_slice().to_vec();
    unsafe { metadata.decref(roc_host) };

    let cell = unsafe { crate::resources::resource_ref::<StreamCell>(handle) };
    if let Some(reader) = cell.reader.take() {
        match cell.pool.take().and_then(|pool| pool.upgrade()) {
            // Not from a pool, or the pool is gone: dropping the reader
            // closes the socket.
            None => drop(reader),
            Some(pool) => {
                if reusable {
                    let mut inner = pool.inner.lock().unwrap_or_else(|p| p.into_inner());
                    inner.idle.push(IdleConn {
                        reader,
                        metadata: metadata_vec,
                        idled_at: Instant::now(),
                    });
                    drop(inner);
                    // An idle connection is capacity too: wake a waiter to
                    // claim it.
                    pool.released.notify_one();
                } else {
                    drop(reader);
                    pool.release_slot();
                }
            }
        }
    }
    release_tcp_stream(handle, roc_host);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Shutdown, TcpListener};

    fn local_server(
        handler: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handler(stream);
        });
        (port, server)
    }

    #[test]
    fn bounded_delimiter_read_stops_at_the_limit() {
        let (port, server) = local_server(|mut stream| {
            stream.write_all(b"delimiter-free").unwrap();
            stream.shutdown(Shutdown::Write).unwrap();
        });
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut reader = BufReader::new(stream);

        assert!(matches!(
            tcp_read_until_impl(&mut reader, b'|', Some(4), None),
            Err(ReadUntilError::LimitExceeded)
        ));
        server.join().unwrap();
    }

    #[test]
    fn stalled_read_respects_the_deadline() {
        let (port, server) = local_server(|_stream| {
            thread::sleep(Duration::from_millis(150));
        });
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut reader = BufReader::new(stream);

        let error = with_read_timeout(&mut reader, 20, |stream, deadline| {
            tcp_read_up_to_impl(stream, 1, Some(deadline))
        })
        .unwrap_err();
        assert!(matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        server.join().unwrap();
    }

    #[test]
    fn exact_read_reports_unexpected_eof() {
        let (port, server) = local_server(|mut stream| {
            stream.write_all(b"hi").unwrap();
            stream.shutdown(Shutdown::Write).unwrap();
        });
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut reader = BufReader::new(stream);

        let error = tcp_read_exactly_impl(&mut reader, 3, None).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        server.join().unwrap();
    }

    #[test]
    fn connect_timeout_includes_localhost_resolution() {
        // Bind through the same hostname the client resolves so both sides use
        // the platform's preferred address family (IPv6 on Windows, IPv4 on
        // some Unix hosts). Otherwise an unavailable first address can consume
        // a deliberately short deadline before the matching address is tried.
        let listener = TcpListener::bind(("localhost", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            listener.accept().unwrap();
        });

        tcp_connect_with_timeout_impl("localhost".to_string(), port, 10_000).unwrap();
        server.join().unwrap();

        let error = tcp_connect_with_timeout_impl("localhost".to_string(), port, 0).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn stalled_connect_respects_the_deadline() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let mut queued_connections = Vec::new();

        loop {
            match TcpStream::connect_timeout(&address, Duration::from_millis(10)) {
                Ok(stream) => queued_connections.push(stream),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    break;
                }
                Err(error) => panic!("could not fill the local TCP accept queue: {error}"),
            }
            assert!(queued_connections.len() < 4_096);
        }

        let started = Instant::now();
        let error = tcp_connect_with_timeout_impl(address.ip().to_string(), address.port(), 20)
            .unwrap_err();
        assert!(matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn zero_write_timeout_fails_before_writing() {
        let (port, server) = local_server(|_stream| {});
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();

        let error = with_write_timeout(&mut stream, 0, |stream, deadline| {
            tcp_write_impl(stream, b"hello", Some(deadline))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        server.join().unwrap();
    }
}

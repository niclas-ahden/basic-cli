use core::mem::ManuallyDrop;
use std::collections::HashMap;
use std::ffi::c_void;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::roc_platform_abi::*;
use crate::{roc_host, roc_u8_list_from_slice};

// The `Host.TcpStream` backing `Tcp.Stream` is represented by the generated glue
// as `*mut u64`: a boxed u64 holding a raw `*mut StreamCell` (a
// `BufReader<TcpStream>` plus the id of the pool it was acquired from, if any).
// The box is refcounted with `allocate_box`/`decref_box_with`; closing the
// socket — and giving a pooled stream's slot back to its pool — happens in
// `drop_tcp_stream` when the last reference is released. `hosted_tcp_pool_release`
// instead EXTRACTS the cell (nulling the box) so the connection can go back
// into the pool's idle set; later uses of a released handle fail with
// `StreamNotFound`. Each host fn that takes a handle calls
// `release_tcp_stream` before returning to balance the incref Roc performs
// when the stream stays live.
//
// `Host.TcpPool` is likewise a boxed u64 holding a key into a global pool
// registry. Pools are host-managed: a bound (`max_connections`) on total
// connections (checked out + idle), a Condvar to wait on when at the cap, and
// an idle set carrying a caller-owned metadata blob per connection (protocol
// libraries persist per-connection session state across checkouts with it).
//
// Errors cross the boundary as a `RocStr` carrying either "ErrorKind::<Variant>"
// (mapped back to a tag union in Tcp.roc) or "UnexpectedEof"; the Roc side parses
// them into `ConnectErr`/`StreamErr`.

/// How long `pool_acquire` waits for a connection to be released when the
/// pool is at `max_connections` before failing with `ErrorKind::TimedOut`.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);

/// Idle connections older than this are closed instead of reused (the
/// server side has likely timed them out anyway).
const IDLE_TTL: Duration = Duration::from_secs(600);

const TCP_BOX_ALIGN: usize = core::mem::align_of::<u64>();

/// What a `Tcp.Stream` box points at.
struct StreamCell {
    reader: BufReader<TcpStream>,
    /// The pool this stream was acquired from, if any. Its connection slot is
    /// freed when the cell is dropped (or the connection is idled instead by
    /// `hosted_tcp_pool_release`).
    pool: Option<u64>,
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

static NEXT_POOL_ID: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(1));
static POOLS: LazyLock<Mutex<HashMap<u64, Arc<PoolEntry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn get_pool(pool_id: u64) -> Option<Arc<PoolEntry>> {
    POOLS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&pool_id)
        .cloned()
}

/// Give a connection slot back to the pool and wake one waiter.
fn release_slot(pool_id: u64) {
    if let Some(pool) = get_pool(pool_id) {
        let mut inner = pool.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner.total = inner.total.saturating_sub(1);
        drop(inner);
        pool.released.notify_one();
    }
}

fn box_tcp_stream(cell: StreamCell, roc_host: &RocHost) -> *mut u64 {
    let raw: *mut StreamCell = Box::into_raw(Box::new(cell));
    let boxed = unsafe { allocate_box(core::mem::size_of::<u64>(), TCP_BOX_ALIGN, false, roc_host) };
    unsafe {
        *(boxed as *mut u64) = raw as u64;
    }
    boxed as *mut u64
}

/// Deref a stream handle. `None` when the cell was extracted by
/// `hosted_tcp_pool_release` (the handle outlived its release).
unsafe fn tcp_stream_ref<'a>(handle: *mut u64) -> Option<&'a mut StreamCell> {
    let raw = *handle as *mut StreamCell;
    if raw.is_null() {
        None
    } else {
        Some(&mut *raw)
    }
}

/// Take ownership of the cell behind a handle, leaving the box nulled so the
/// destructor (and any stale references) see an already-released stream.
unsafe fn take_tcp_stream(handle: *mut u64) -> Option<Box<StreamCell>> {
    let raw = *handle as *mut StreamCell;
    if raw.is_null() {
        None
    } else {
        *handle = 0;
        Some(Box::from_raw(raw))
    }
}

extern "C" fn drop_tcp_stream(data_ptr: *mut c_void, _roc_host: *mut RocHost) {
    unsafe {
        let raw = *(data_ptr as *mut u64) as *mut StreamCell;
        if !raw.is_null() {
            let cell = Box::from_raw(raw);
            if let Some(pool_id) = cell.pool {
                release_slot(pool_id);
            }
            drop(cell);
        }
    }
}

fn release_tcp_stream(handle: *mut u64, roc_host: &RocHost) {
    unsafe {
        decref_box_with(
            handle as RocBox,
            TCP_BOX_ALIGN,
            false,
            Some(drop_tcp_stream),
            roc_host,
        )
    };
}

extern "C" fn drop_tcp_pool(data_ptr: *mut c_void, _roc_host: *mut RocHost) {
    unsafe {
        let pool_id = *(data_ptr as *mut u64);
        // Removing the entry drops the idle connections; checked-out streams
        // keep working and their slot releases become no-ops.
        POOLS
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&pool_id);
    }
}

fn release_tcp_pool(handle: *mut u64, roc_host: &RocHost) {
    unsafe {
        decref_box_with(
            handle as RocBox,
            TCP_BOX_ALIGN,
            false,
            Some(drop_tcp_pool),
            roc_host,
        )
    };
}

fn stream_not_found(roc_host: &RocHost) -> RocStr {
    RocStr::from_str("StreamNotFound", roc_host)
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
pub extern "C" fn hosted_tcp_connect(host: RocStr, port: u16) -> HostTcpConnectResult {
    let roc_host = roc_host();
    let host_string = host.as_str().to_owned();
    unsafe { host.decref(roc_host) };

    match TcpStream::connect((host_string.as_str(), port)) {
        Ok(stream) => {
            let handle = box_tcp_stream(
                StreamCell {
                    reader: BufReader::new(stream),
                    pool: None,
                },
                roc_host,
            );
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
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_read_err(stream_not_found(roc_host)),
        Some(cell) => {
            let stream = &mut cell.reader;
            let mut chunk = stream.take(bytes_to_read);
            match chunk.fill_buf() {
                Ok(received) => {
                    let received = received.to_vec();
                    stream.consume(received.len());
                    try_tcp_read_ok(roc_u8_list_from_slice(&received, roc_host))
                }
                Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
            }
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
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_read_err(stream_not_found(roc_host)),
        Some(cell) => {
            let stream = &mut cell.reader;
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
        }
    };
    release_tcp_stream(handle, roc_host);
    result
}

#[no_mangle]
pub extern "C" fn hosted_tcp_read_until(handle: *mut u64, byte: u8) -> HostTcpReadUntilResult {
    let roc_host = roc_host();
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_read_err(stream_not_found(roc_host)),
        Some(cell) => match tcp_read_until_impl(&mut cell.reader, byte) {
            Ok(buffer) => try_tcp_read_ok(roc_u8_list_from_slice(&buffer, roc_host)),
            Err(err) => try_tcp_read_err(to_tcp_stream_err(err, roc_host)),
        },
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
    let result = match unsafe { tcp_stream_ref(handle) } {
        None => try_tcp_write_err(stream_not_found(roc_host)),
        Some(cell) => match cell.reader.get_mut().write_all(msg.as_slice()) {
            Ok(()) => try_tcp_write_ok(),
            Err(err) => try_tcp_write_err(to_tcp_stream_err(err, roc_host)),
        },
    };
    unsafe { msg.decref(roc_host) };
    release_tcp_stream(handle, roc_host);
    result
}

/// Shut the socket down in both directions immediately. Resources (and the
/// pool slot, for pooled streams) are still freed by the box destructor when
/// the last reference drops.
#[no_mangle]
pub extern "C" fn hosted_tcp_shutdown(handle: *mut u64) {
    let roc_host = roc_host();
    if let Some(cell) = unsafe { tcp_stream_ref(handle) } {
        let _ = cell.reader.get_ref().shutdown(Shutdown::Both);
    }
    release_tcp_stream(handle, roc_host);
}

// ============================================================================
// Connection pools
// ============================================================================

#[no_mangle]
pub extern "C" fn hosted_tcp_pool_create(host: RocStr, port: u16, max_connections: u64) -> *mut u64 {
    let roc_host = roc_host();
    let host_string = host.as_str().to_owned();
    unsafe { host.decref(roc_host) };

    let pool_id = {
        let mut next_id = NEXT_POOL_ID.lock().unwrap_or_else(|p| p.into_inner());
        let id = *next_id;
        *next_id += 1;
        id
    };
    POOLS.lock().unwrap_or_else(|p| p.into_inner()).insert(
        pool_id,
        Arc::new(PoolEntry {
            inner: Mutex::new(PoolInner {
                host: host_string,
                port,
                max_connections: max_connections.max(1) as usize,
                idle: Vec::new(),
                total: 0,
            }),
            released: Condvar::new(),
        }),
    );

    let boxed = unsafe { allocate_box(core::mem::size_of::<u64>(), TCP_BOX_ALIGN, false, roc_host) };
    unsafe {
        *(boxed as *mut u64) = pool_id;
    }
    boxed as *mut u64
}

/// True if a pooled idle connection is still usable: readable with no
/// buffered protocol bytes means the peer closed or desynced; `WouldBlock`
/// means healthy-and-idle.
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
        Ok(0) => false,                                      // peer closed
        Ok(_) => false,                                      // unexpected bytes: desync
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
    let pool_id = unsafe { *pool_handle };
    // Clone the pool's Arc BEFORE releasing our owned handle reference. On the
    // Roc `Pool` value's last use, Roc hands ownership over without an incref,
    // so `release_tcp_pool` drops the box to a zero refcount and runs
    // `drop_tcp_pool`, which removes the registry entry. Looking the pool up
    // first keeps it alive for the duration of this acquire (mirroring the
    // "use the handle, then release it last" order the stream ops use).
    let pool = get_pool(pool_id);
    release_tcp_pool(pool_handle, roc_host);

    let pool = match pool {
        Some(pool) => pool,
        None => {
            return try_pool_acquire_err(RocStr::from_str("Unrecognized: pool not found", roc_host))
        }
    };
    let deadline = Instant::now() + ACQUIRE_TIMEOUT;

    let mut inner = pool.inner.lock().unwrap_or_else(|p| p.into_inner());
    loop {
        // Recycled connection if a live one is idle (dead/stale ones free
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
            let handle = box_tcp_stream(
                StreamCell {
                    reader,
                    pool: Some(pool_id),
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
        // no lock is held during connect).
        if inner.total < inner.max_connections {
            inner.total += 1;
            let host = inner.host.clone();
            let port = inner.port;
            drop(inner);
            match TcpStream::connect((host.as_str(), port)) {
                Ok(stream) => {
                    let handle = box_tcp_stream(
                        StreamCell {
                            reader: BufReader::new(stream),
                            pool: Some(pool_id),
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
                    release_slot(pool_id); // give the reserved slot back
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

    if let Some(cell) = unsafe { take_tcp_stream(handle) } {
        let StreamCell { reader, pool } = *cell;
        match pool {
            None => {} // not from a pool: drop closes the socket
            Some(pool_id) => {
                if !reusable {
                    release_slot(pool_id); // drop closes the socket
                } else {
                    match get_pool(pool_id) {
                        Some(pool) => {
                            let mut inner = pool.inner.lock().unwrap_or_else(|p| p.into_inner());
                            inner.idle.push(IdleConn {
                                reader,
                                metadata: metadata_vec,
                                idled_at: Instant::now(),
                            });
                            drop(inner);
                            // An idle connection is capacity too: wake a
                            // waiter to claim it.
                            pool.released.notify_one();
                        }
                        None => {} // pool gone: drop closes the socket
                    }
                }
            }
        }
    }
    release_tcp_stream(handle, roc_host);
}

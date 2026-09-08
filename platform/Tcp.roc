import Host

## Connect to TCP servers and exchange buffered byte streams.
##
## See the [host runtime behavior](https://github.com/roc-lang/basic-cli#host-runtime-behavior)
## for additional runtime details.
##
## Timeout arguments are in milliseconds and apply to the whole operation,
## including every underlying read/write attempt. A zero timeout fails
## immediately. Read and write timeouts are returned as
## `TcpReadErr(TimedOut)` and `TcpWriteErr(TimedOut)` respectively.
##
## ```roc
## stream = Tcp.connect!("example.com", 80, 5_000)?
## stream.write_utf8!("GET / HTTP/1.0\r\nHost: example.com\r\n\r\n", 5_000)?
## status_line = stream.read_line!(1_024, 5_000)?
## ```
Tcp :: [].{

	## A listening socket. Final ARC release closes it; close! affects all aliases.
	Listener :: { host : Host.TcpListener }.{
		to_inspect : Listener -> Str
		to_inspect = |_| "Tcp.Listener(<opaque>)"

		## Read the reserved local port.
		local_port! : Listener => Try(U16, _)
		local_port! = |listener| Host.tcp_local_port!(listener.host).map_err(parse_listener_err)

		## Accept a connection within the given whole-operation timeout.
		accept! : Listener, U64 => Try(Stream, _)
		accept! = |listener, timeout_ms|
			Host.tcp_accept!(listener.host, timeout_ms)
				.map_ok(|host| Stream.{ host })
				.map_err(parse_listener_err)

		## Close the listener. Repeated closes succeed.
		close! : Listener => Try({}, _)
		close! = |listener| Host.tcp_listener_close!(listener.host).map_err(parse_listener_err)
	}

	## Bind an address and listen. Port zero reserves an OS-assigned port until
	## this listener is closed. The timeout includes hostname resolution.
	listen! : Str, U16, U64 => Try(Listener, _)
	listen! = |host, port, timeout_ms|
		Host.tcp_listen!(host, port, timeout_ms)
			.map_ok(|host_handle| Listener.{ host: host_handle })
			.map_err(parse_listener_err)

	## Represents a TCP stream.
	##
	## The connection is automatically closed when the last reference to the
	## stream is dropped. It wraps an opaque host-side `BufReader<TcpStream>`
	## handle.
	Stream :: { host : Host.TcpStream }.{

		## Render the stream without exposing its host handle.
		to_inspect : Stream -> Str
		to_inspect = |_| "Tcp.Stream(<opaque>)"

		## Read up to a number of bytes, waiting at most `timeout_ms` milliseconds.
		read_up_to! : Stream, U64, U64 => Try(List(U8), _)
		read_up_to! = |stream, bytes_to_read, timeout_ms|
			Host.tcp_read_up_to!(stream.host, bytes_to_read, timeout_ms)
				.map_err(|err| TcpReadErr(parse_stream_err(err)))

		## Read an exact number of bytes, waiting at most `timeout_ms` milliseconds.
		##
		## `TcpUnexpectedEOF` is returned if the stream ends before the specified
		## number of bytes is reached.
		read_exactly! : Stream, U64, U64 => Try(List(U8), _)
		read_exactly! = |stream, bytes_to_read, timeout_ms|
			match Host.tcp_read_exactly!(stream.host, bytes_to_read, timeout_ms) {
				Ok(bytes) => Ok(bytes)
				Err("UnexpectedEof") => Err(TcpUnexpectedEOF)
				Err(err) => Err(TcpReadErr(parse_stream_err(err)))
			}

		## Read until a delimiter or EOF is reached, consuming at most `max_bytes`
		## and waiting at most `timeout_ms` milliseconds.
		## If found, the delimiter is included as the last byte. Returns
		## `TcpReadLimitExceeded(max_bytes)` if the delimiter was not found within
		## the limit.
		read_until! : Stream, U8, U64, U64 => Try(List(U8), _)
		read_until! = |stream, byte, max_bytes, timeout_ms|
			match Host.tcp_read_until!(stream.host, byte, max_bytes, timeout_ms) {
				Ok(bytes) => Ok(bytes)
				Err("LimitExceeded") => Err(TcpReadLimitExceeded(max_bytes))
				Err(err) => Err(TcpReadErr(parse_stream_err(err)))
			}

		## Read at most `max_bytes` through a newline (`\n`, byte 10) or EOF as
		## UTF-8, waiting at most `timeout_ms` milliseconds. If found, the newline
		## is included as the last character.
		read_line! : Stream, U64, U64 => Try(Str, _)
		read_line! = |stream, max_bytes, timeout_ms|
			match read_until!(stream, 10, max_bytes, timeout_ms) {
				Ok(bytes) => Str.from_utf8(bytes).map_err(|err| TcpReadBadUtf8(err))
				Err(err) => Err(err)
			}

		## Write bytes, waiting at most `timeout_ms` milliseconds.
		write! : Stream, List(U8), U64 => Try({}, _)
		write! = |stream, bytes, timeout_ms|
			Host.tcp_write!(stream.host, bytes, timeout_ms)
				.map_err(|err| TcpWriteErr(parse_stream_err(err)))

		## Write a string as UTF-8, waiting at most `timeout_ms` milliseconds.
		write_utf8! : Stream, Str, U64 => Try({}, _)
		write_utf8! = |stream, str, timeout_ms|
			write!(stream, Str.to_utf8(str), timeout_ms)
	}

	## A host-managed pool of TCP connections to one address (see [Tcp.pool!]).
	Pool :: { host : Host.TcpPool }.{

		## Render the pool without exposing its host handle.
		to_inspect : Pool -> Str
		to_inspect = |_| "Tcp.Pool(<opaque>)"
	}

	## Represents errors that can occur when connecting to a remote host.
	ConnectErr : [
		PermissionDenied,
		AddrInUse,
		AddrNotAvailable,
		ConnectionRefused,
		Interrupted,
		TimedOut,
		Unsupported,
		Unrecognized(Str),
	]

	## Represents errors that can occur when performing an effect with a `Stream`.
	StreamErr : [
		StreamNotFound,
		PermissionDenied,
		ConnectionRefused,
		ConnectionReset,
		Interrupted,
		TimedOut,
		OutOfMemory,
		BrokenPipe,
		Unrecognized(Str),
	]

	## Opens a TCP connection, waiting at most `timeout_ms` milliseconds across
	## DNS resolution and all resolved addresses. A zero timeout fails immediately.
	##
	## ```roc
	## # Connect to localhost:8080
	## stream = Tcp.connect!("localhost", 8080, 5_000)?
	## ```
	##
	## Valid hostnames look like `127.0.0.1`, `::1`, `localhost`, or `roc-lang.org`.
	connect! = |host, port, timeout_ms|
		Host.tcp_connect!(host, port, timeout_ms)
			.map_ok(|stream| Stream.{ host: stream })
			.map_err(parse_connect_err)

	## Close a TCP stream immediately by shutting the socket down in both
	## directions. The underlying resources are freed when the last reference
	## to the stream is dropped, but the shutdown happens right away. Use this
	## to abandon a connection that is in an unknown protocol state, for
	## example after an error mid-conversation, especially for streams
	## acquired from a [Pool].
	close! : Stream => {}
	close! = |stream| Host.tcp_shutdown!(stream.host)

	## Create a connection pool for the given address. Creating a pool does not
	## connect. Connections are dialed lazily by [Tcp.pool_acquire!].
	##
	## `max_connections` bounds the TOTAL number of connections the pool will
	## have open at once (checked out + idle), like Axum/sqlx's
	## `max_connections`. When the pool is at the cap, [Tcp.pool_acquire!]
	## waits for a release instead of dialing, and fails with
	## `TcpConnectErr(TimedOut)` if none frees up within 30 seconds. Idle
	## connections unused for 10 minutes are closed and re-dialed on demand.
	##
	## The `Pool` value is an immutable handle to host-managed state, so it
	## can be passed around freely and acquired from concurrently.
	pool! : { host : Str, port : U16, max_connections : U64 } => Pool
	pool! = |{ host, port, max_connections }|
		Pool.{ host: Host.tcp_pool_create!(host, port, max_connections) }

	## Check a connection out of the pool.
	##
	## Returns a recycled connection (`fresh: Bool.False`, plus whatever
	## `metadata` it was released with) when one is available, otherwise dials
	## a new one (`fresh: Bool.True`, empty `metadata`). If the pool is at
	## `max_connections`, waits up to 30s for a release, then fails with
	## `TcpConnectErr(TimedOut)`.
	##
	## `metadata` is a caller-owned blob stored with the idle connection at
	## [Tcp.pool_release!] time. Protocol libraries use it to persist
	## per-connection session state (e.g. Postgres backend keys) across
	## checkouts.
	##
	## Every acquired stream should be either released with [Tcp.pool_release!]
	## (to be reused) or dropped or closed with [Tcp.close!]. A dropped stream
	## frees its pool slot when the last reference goes away.
	pool_acquire! : Pool => Try({ stream : Stream, fresh : Bool, metadata : List(U8) }, [TcpConnectErr(ConnectErr), ..])
	pool_acquire! = |pool|
		match Host.tcp_pool_acquire!(pool.host) {
			Ok(acquired) => Ok({ stream: Stream.{ host: acquired.stream }, fresh: acquired.fresh, metadata: acquired.metadata })
			Err(err) => Err(TcpConnectErr(parse_connect_err(err)))
		}

	## Return a connection to its pool for another checkout to reuse, storing
	## `metadata` alongside it. Only release connections that are in a
	## known-good protocol state. After an error mid-conversation, use
	## [Tcp.close!] instead.
	pool_release! : { stream : Stream, metadata : List(U8) } => {}
	pool_release! = |{ stream, metadata }|
		Host.tcp_pool_release!(stream.host, Bool.True, metadata)

	## Convert a `ConnectErr` to a `Str` you can print.
	connect_err_to_str = |err|
		match err {
			PermissionDenied => "PermissionDenied"
			AddrInUse => "AddrInUse"
			AddrNotAvailable => "AddrNotAvailable"
			ConnectionRefused => "ConnectionRefused"
			Interrupted => "Interrupted"
			TimedOut => "TimedOut"
			Unsupported => "Unsupported"
			Unrecognized(message) => "Unrecognized Error: ${message}"
		}

	## Convert a `StreamErr` to a `Str` you can print.
	stream_err_to_str = |err|
		match err {
			StreamNotFound => "StreamNotFound"
			PermissionDenied => "PermissionDenied"
			ConnectionRefused => "ConnectionRefused"
			ConnectionReset => "ConnectionReset"
			Interrupted => "Interrupted"
			TimedOut => "TimedOut"
			OutOfMemory => "OutOfMemory"
			BrokenPipe => "BrokenPipe"
			Unrecognized(message) => "Unrecognized Error: ${message}"
		}
}

# ---- internal helpers (module-private) -----------------------------------------

parse_connect_err = |err|
	match err {
		"ErrorKind::PermissionDenied" => PermissionDenied
		"ErrorKind::AddrInUse" => AddrInUse
		"ErrorKind::AddrNotAvailable" => AddrNotAvailable
		"ErrorKind::ConnectionRefused" => ConnectionRefused
		"ErrorKind::Interrupted" => Interrupted
		"ErrorKind::TimedOut" => TimedOut
		"ErrorKind::Unsupported" => Unsupported
		other => Unrecognized(other)
	}

parse_stream_err = |err|
	match err {
		"StreamNotFound" => StreamNotFound
		"ErrorKind::PermissionDenied" => PermissionDenied
		"ErrorKind::ConnectionRefused" => ConnectionRefused
		"ErrorKind::ConnectionReset" => ConnectionReset
		"ErrorKind::Interrupted" => Interrupted
		"ErrorKind::TimedOut" => TimedOut
		"ErrorKind::OutOfMemory" => OutOfMemory
		"ErrorKind::BrokenPipe" => BrokenPipe
		other => Unrecognized(other)
	}

expect parse_connect_err("ErrorKind::TimedOut") == TimedOut

expect parse_stream_err("ErrorKind::TimedOut") == TimedOut

parse_listener_err = |err|
	match err {
		"ListenerClosed" => ListenerClosed
		_ => TcpListenErr(parse_connect_err(err))
	}

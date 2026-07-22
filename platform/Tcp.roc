import Host

## Connect to TCP servers and exchange buffered byte streams.
##
## See the [host runtime behavior](https://github.com/roc-lang/basic-cli#host-runtime-behavior)
## for current timeout and buffering limitations.
Tcp :: [].{

	## Represents a TCP stream.
	##
	## The connection is automatically closed when the last reference to the
	## stream is dropped. It wraps an opaque host-side `BufReader<TcpStream>`
	## handle.
	Stream :: { host : Host.TcpStream }.{

		## Render the stream without exposing its host handle.
		to_inspect : Stream -> Str
		to_inspect = |_| "Tcp.Stream(<opaque>)"

		## Read up to a number of bytes from this TCP stream.
		read_up_to! : Stream, U64 => Try(List(U8), _)
		read_up_to! = |stream, bytes_to_read|
			Host.tcp_read_up_to!(stream.host, bytes_to_read)
				.map_err(|err| TcpReadErr(parse_stream_err(err)))

		## Read an exact number of bytes or fail.
		##
		## `TcpUnexpectedEOF` is returned if the stream ends before the specified
		## number of bytes is reached.
		read_exactly! : Stream, U64 => Try(List(U8), _)
		read_exactly! = |stream, bytes_to_read|
			match Host.tcp_read_exactly!(stream.host, bytes_to_read) {
				Ok(bytes) => Ok(bytes)
				Err("UnexpectedEof") => Err(TcpUnexpectedEOF)
				Err(err) => Err(TcpReadErr(parse_stream_err(err)))
			}

		## Read until a delimiter or EOF is reached. If found, the delimiter is
		## included as the last byte.
		read_until! : Stream, U8 => Try(List(U8), _)
		read_until! = |stream, byte|
			Host.tcp_read_until!(stream.host, byte)
				.map_err(|err| TcpReadErr(parse_stream_err(err)))

		## Read until a newline (`\n`, byte 10) or EOF is reached as UTF-8.
		## If found, the newline is included as the last character.
		read_line! : Stream => Try(Str, _)
		read_line! = |stream|
		# NB: use `match` rather than `?` here — `read_until!` yields a
		# single-variant error union and `?` currently miscompiles (roc#9826).
			match read_until!(stream, 10) {
				Ok(bytes) => Str.from_utf8(bytes).map_err(|err| TcpReadBadUtf8(err))
				Err(err) => Err(err)
			}

		## Write bytes to this TCP stream.
		write! : Stream, List(U8) => Try({}, _)
		write! = |stream, bytes|
			Host.tcp_write!(stream.host, bytes)
				.map_err(|err| TcpWriteErr(parse_stream_err(err)))

		## Write a string to this TCP stream, encoded as UTF-8.
		write_utf8! : Stream, Str => Try({}, _)
		write_utf8! = |stream, str| write!(stream, Str.to_utf8(str))
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
		OutOfMemory,
		BrokenPipe,
		Unrecognized(Str),
	]

	## Opens a TCP connection to a remote host.
	##
	## ```roc
	## # Connect to localhost:8080
	## stream = Tcp.connect!("localhost", 8080)?
	## ```
	##
	## Valid hostnames look like `127.0.0.1`, `::1`, `localhost`, or `roc-lang.org`.
	connect! = |host, port|
		Host.tcp_connect!(host, port)
			.map_ok(|stream| Stream.{ host: stream })
			.map_err(parse_connect_err)

	## Close a TCP stream immediately by shutting the socket down in both
	## directions. The underlying resources are freed when the last reference
	## to the stream is dropped, but shutdown happens right away — use this to
	## abandon a connection that is in an unknown protocol state (e.g. after an
	## error mid-conversation), especially for streams acquired from a [Pool].
	close! : Stream => {}
	close! = |stream| Host.tcp_shutdown!(stream.host)

	## Create a connection pool for the given address. Creating a pool does not
	## connect; connections are dialed lazily by [Tcp.pool_acquire!].
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
	## Returns a recycled connection (`fresh: Bool.false`, plus whatever
	## `metadata` it was released with) when one is available, otherwise dials
	## a new one (`fresh: Bool.true`, empty `metadata`). If the pool is at
	## `max_connections`, waits up to 30s for a release, then fails with
	## `TcpConnectErr(TimedOut)`.
	##
	## `metadata` is a caller-owned blob stored with the idle connection at
	## [Tcp.pool_release!] time — protocol libraries use it to persist
	## per-connection session state (e.g. Postgres backend keys) across
	## checkouts.
	##
	## Every acquired stream should be either [Tcp.pool_release!]d (to be
	## reused) or dropped/[Tcp.close!]d — a dropped stream frees its pool slot
	## when the last reference goes away.
	pool_acquire! : Pool => Try({ stream : Stream, fresh : Bool, metadata : List(U8) }, [TcpConnectErr(ConnectErr), ..])
	pool_acquire! = |pool|
		match Host.tcp_pool_acquire!(pool.host) {
			Ok(acquired) => Ok({ stream: Stream.{ host: acquired.stream }, fresh: acquired.fresh, metadata: acquired.metadata })
			Err(err) => Err(TcpConnectErr(parse_connect_err(err)))
		}

	## Return a connection to its pool for another checkout to reuse, storing
	## `metadata` alongside it. Only release connections that are in a
	## known-good protocol state; after an error mid-conversation, use
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
		"ErrorKind::OutOfMemory" => OutOfMemory
		"ErrorKind::BrokenPipe" => BrokenPipe
		other => Unrecognized(other)
	}

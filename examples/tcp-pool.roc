app [main!] { pf: platform "https://github.com/niclas-ahden/basic-cli/releases/download/0.22.0/7FuVkuWGpyRSLu5w8vfBx82bhqpiVRv1gqQZcrA2H9m9.tar.zst" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Tcp

# Exercises the TCP connection pool against a local echo server:
#
#   python3 scripts/tcp_echo_server.py &   # listens on 127.0.0.1:8085
#   roc examples/tcp-pool.roc
#
# Acquires dial lazily (fresh: True), releases park the connection in the
# pool's idle set together with a caller-owned metadata blob, and the next
# acquire gets the same connection back (fresh: False) with that metadata.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	pool = Tcp.pool!({ host: "127.0.0.1", port: 8085, max_connections: 2 })

	# First checkout dials a fresh connection: no metadata yet.
	first = Tcp.pool_acquire!(pool)?
	Stdout.line!("first acquire: fresh=${Str.inspect(first.fresh)} metadata=${Str.inspect(first.metadata)}")?

	Tcp.Stream.write!(first.stream, Str.to_utf8("hello pool\n"))?
	echoed = Tcp.Stream.read_until!(first.stream, 10)?
	Stdout.line!("echo said: ${Str.from_utf8_lossy(echoed).trim()}")?

	# Release in a known-good state, parking session metadata alongside it.
	Tcp.pool_release!({ stream: first.stream, metadata: [42, 43] })

	# Second checkout recycles the released connection, metadata intact.
	second = Tcp.pool_acquire!(pool)?
	Stdout.line!("second acquire: fresh=${Str.inspect(second.fresh)} metadata=${Str.inspect(second.metadata)}")?

	Tcp.Stream.write!(second.stream, Str.to_utf8("still the same conn\n"))?
	echoed2 = Tcp.Stream.read_until!(second.stream, 10)?
	Stdout.line!("echo said: ${Str.from_utf8_lossy(echoed2).trim()}")?

	# A connection in an unknown protocol state should be closed, not
	# released: the pool slot frees up when the stream is dropped.
	Tcp.close!(second.stream)

	Stdout.line!("tcp pool ok")?
	Ok({})
}

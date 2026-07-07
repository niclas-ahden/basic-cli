app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Tcp
import pf.Stdout
import pf.Stdin

# Simple TCP client in Roc.
#
# Connects to a server on localhost:8085, reads user input from stdin, sends it
# to the server, and prints the server's response — looping until end-of-input.
#
# To try it interactively, start an echo server in another terminal first:
#
#     $ ncat -e $(which cat) -l 8085
#
# then run this example.
main! : List(OsStr) => Try({}, _)
main! = |_args| {
    stream = Tcp.connect!("127.0.0.1", 8085) ? |err| ConnectFailed(err)
    Stdout.line!("Connected!")?
    run!(stream)
}

## Read a line from stdin, send it to the server, print the response, repeat.
run! : Tcp.Stream => Try({}, _)
run! = |stream| {
    Stdout.write!("> ")?
    match Stdin.line!() {
        # No more input — exit cleanly.
        Err(EndOfFile) => Ok({})
        Err(StdinErr(err)) => Err(StdinReadFailed(err))
        Ok(out_msg) => {
            Tcp.write_utf8!(stream, "${out_msg}\n") ? |err| TcpWriteFailed(err)
            in_msg = Tcp.read_line!(stream) ? |err| TcpReadFailed(err)
            Stdout.line!("< ${in_msg}")?
            run!(stream)
        }
    }
}

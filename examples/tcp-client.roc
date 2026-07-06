app [main!] { pf: platform "../platform/main.roc" }

import pf.Tcp
import pf.Stdout
import pf.Stdin
import pf.Stderr

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
main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
    match Tcp.connect!("127.0.0.1", 8085) {
        Ok(stream) => {
            Stdout.line!("Connected!") ? |_| Exit(1)
            run!(stream)
        }
        Err(connect_err) => report_connect_err!(connect_err)
    }

## Read a line from stdin, send it to the server, print the response, repeat.
run! : Tcp.Stream => Try({}, [Exit(I32), ..])
run! = |stream| {
    Stdout.write!("> ") ? |_| Exit(1)
    match Stdin.line!() {
        # No more input — exit cleanly.
        Err(EndOfFile) => Ok({})
        Err(StdinErr(_)) => Ok({})
        Ok(out_msg) =>
            match Tcp.write_utf8!(stream, "${out_msg}\n") {
                Err(TcpWriteErr(err)) => report_stream_err!("writing", err)
                Ok({}) =>
                    match Tcp.read_line!(stream) {
                        Err(read_err) => report_read_err!(read_err)
                        Ok(in_msg) => {
                            Stdout.line!("< ${in_msg}") ? |_| Exit(1)
                            run!(stream)
                        }
                    }
            }
    }
}

report_connect_err! : Tcp.ConnectErr => Try({}, [Exit(I32), ..])
report_connect_err! = |err| {
    err_str = Tcp.connect_err_to_str(err)
    Stderr.line!(
        \\Failed to connect: ${err_str}
        \\
        \\If you don't have anything listening on port 8085, run:
        \\    $ nc -l 8085
        \\
        \\If you want an echo server you can run:
        \\    $ ncat -e $(which cat) -l 8085
    ) ? |_| Exit(1)
    Ok({})
}

report_read_err! : [TcpReadErr(Tcp.StreamErr), TcpReadBadUtf8(_)] => Try({}, [Exit(I32), ..])
report_read_err! = |err|
    match err {
        TcpReadErr(stream_err) => report_stream_err!("reading", stream_err)
        TcpReadBadUtf8(_) => {
            Stderr.line!("Received invalid UTF-8 data") ? |_| Exit(1)
            Ok({})
        }
    }

report_stream_err! : Str, Tcp.StreamErr => Try({}, [Exit(I32), ..])
report_stream_err! = |action, err| {
    err_str = Tcp.stream_err_to_str(err)
    Stderr.line!("Error while ${action}: ${err_str}") ? |_| Exit(1)
    Ok({})
}

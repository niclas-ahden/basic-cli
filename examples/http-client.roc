app [main!] { pf: platform "../platform/main.roc" }

import pf.Http
import pf.Stdout

# Minimal HTTP client demo: performs an HTTPS GET and reports the result.
#
#     roc build examples/http-client.roc
#     ./examples/http-client
main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
    match Http.get_utf8!("https://example.com") {
        Ok(body) => {
            byte_count = List.len(Str.to_utf8(body))
            _ = Stdout.line!("Fetched https://example.com over HTTPS (${U64.to_str(byte_count)} bytes).")
            Ok({})
        }
        Err(HttpErr(Timeout)) => report_failure!("request timed out")
        Err(HttpErr(NetworkError)) => report_failure!("network error")
        Err(HttpErr(BadBody)) => report_failure!("could not read response body")
        Err(HttpErr(Other(_))) => report_failure!("transport error")
        Err(BadBody(message)) => report_failure!(message)
    }

report_failure! : Str => Try({}, [Exit(I32), ..])
report_failure! = |message| {
    _ = Stdout.line!("HTTP request failed: ${message}")
    Ok({})
}

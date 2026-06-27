app [main!] {
    pf: platform "../platform/main.roc",
    http: "https://github.com/roc-lang/http/releases/download/0.1/6LcdNq2r7xTBwj972ecYWUkMWobJr94yL2NyJpHRAXap.tar.zst",
}

import pf.Http
import pf.Stdout
import http.Request
import http.Response

# Demo of the basic-cli HTTP client against a local server.
#
# To run this example, first start the test server in another terminal:
#
#     cd ci/rust_http_server && cargo run --release
#
# then:
#
#     roc build examples/http-client.roc
#     ./examples/http-client
main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
    # GET a plain-text body and decode it as UTF-8.
    match Http.get_utf8!("http://localhost:9000/utf8test") {
        Err(_) => report_failure!("GET /utf8test failed")
        Ok(utf8) => {
            _ = Stdout.line!("I received '${utf8}' from the server.")

            # GET a JSON body (returned here as the raw response string).
            match Http.get_utf8!("http://localhost:9000") {
                Err(_) => report_failure!("GET / failed")
                Ok(json) => {
                    _ = Stdout.line!("The json I received was: ${json}")

                    # Use send! with a custom header and inspect the Response.
                    request0 = Request.from_method(GET)
                    request1 = Request.with_uri(request0, "http://localhost:9000/utf8test")
                    request = Request.add_header(request1, "Accept", "text/plain")
                    match Http.send!(request) {
                        Ok(response) => {
                            _ = Stdout.line!("send! returned status ${U16.to_str(Response.status(response))}.")
                            Ok({})
                        }
                        Err(HttpErr(_)) => report_failure!("send! failed")
                    }
                }
            }
        }
    }

report_failure! : Str => Try({}, [Exit(I32), ..])
report_failure! = |message| {
    _ = Stdout.line!("HTTP request failed: ${message}")
    Ok({})
}

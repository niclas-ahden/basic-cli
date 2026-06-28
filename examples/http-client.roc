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

            # Use send! directly and inspect the Response.
            request0 = Request.from_method(GET)
            request = Request.with_uri(request0, "http://localhost:9000/utf8test")
            match Http.send!(request) {
                Err(HttpErr(_)) => report_failure!("send! failed")
                Ok(response) => {
                    status = U16.to_str(Response.status(response))

                    # GET a JSON body and decode it into a Roc record.
                    json_result : Try({ foo : Str }, _)
                    json_result = Http.get!("http://localhost:9000")
                    match json_result {
                        Err(_) => report_failure!("GET / failed")
                        Ok(decoded) => {
                            _ = Stdout.line!("The json I received was: { foo: \"${decoded.foo}\" }")
                            _ = Stdout.line!("send! returned status ${status}.")
                            Ok({})
                        }
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

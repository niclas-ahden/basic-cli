app [main!] {
    pf: platform "../platform/main.roc",
    http: "https://github.com/roc-lang/http/releases/download/1.0.0/6ZUwqYhCS8PU9Mo6MF7oV82ET2o7KYb57CLKDq4cq4sS.tar.zst",
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
    match run_demo!() {
        Ok({}) => Ok({})
        Err(message) => report_failure!(message)
    }

report_failure! : Str => Try({}, [Exit(I32), ..])
report_failure! = |message| {
    Stdout.line!("HTTP request failed: ${message}") ? |_| Exit(1)
    Err(Exit(1))
}

run_demo! : () => Try({}, Str)
run_demo! = || {
    utf8 = Http.get_utf8!("http://localhost:9000/utf8test") ? |_| "GET /utf8test failed"
    write_line!("I received '${utf8}' from the server.")?

    request = Request.from_method(GET).with_uri("http://localhost:9000/utf8test")
    response = Http.send!(request) ? |_| "send! failed"
    status = U16.to_str(Response.status(response))

    decoded : { foo : Str }
    decoded = Http.get!("http://localhost:9000") ? |_| "GET / failed"

    write_line!("The json I received was: { foo: \"${decoded.foo}\" }")?
    write_line!("send! returned status ${status}.")?
    reject_invalid_json!()?
    reject_invalid_utf8!()?

    Ok({})
}

reject_invalid_json! : () => Try({}, Str)
reject_invalid_json! = || {
    result : Try({ foo : Str }, _)
    result = Http.get!("http://localhost:9000/invalid-json")

    match result {
        Err(JsonErr(_)) => write_line!("invalid JSON was rejected.")
        Err(_) => Err("GET /invalid-json failed with the wrong error")
        Ok(_) => Err("GET /invalid-json unexpectedly succeeded")
    }
}

reject_invalid_utf8! : () => Try({}, Str)
reject_invalid_utf8! = || {
    result = Http.get_utf8!("http://localhost:9000/invalid-utf8")

    match result {
        Err(BadBody(_)) => write_line!("invalid UTF-8 was rejected.")
        Err(_) => Err("GET /invalid-utf8 failed with the wrong error")
        Ok(_) => Err("GET /invalid-utf8 unexpectedly succeeded")
    }
}

write_line! : Str => Try({}, Str)
write_line! = |message| {
    Stdout.line!(message) ? |_| "stdout write failed"
    Ok({})
}

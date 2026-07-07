app [main!] {
    pf: platform "../platform/main.roc",
    http: "https://github.com/roc-lang/http/releases/download/1.0.0/6ZUwqYhCS8PU9Mo6MF7oV82ET2o7KYb57CLKDq4cq4sS.tar.zst",
}

import pf.Http
import pf.Stdout
import http.Request
import http.Response

# Demo of the basic-cli HTTP functions against a local server.

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
    match run_demo!() {
        Ok({}) => Ok({})
        Err(message) => {
            Stdout.line!("HTTP example failed: ${message}") ? |_| Exit(1)
            Err(Exit(1))
        }
    }

run_demo! : () => Try({}, Str)
run_demo! = || {
    hello_str = Http.get_utf8!("http://localhost:9000/utf8test") ? |_| "GET /utf8test failed"
    write_line!("I received '${hello_str}' from the server.")?

    decoded : { foo : Str }
    decoded = Http.get!("http://localhost:9000") ? |_| "GET / failed"

    write_line!("The json I received was: { foo: \"${decoded.foo}\" }")?

    response = Http.send!(Request.from_method(GET).with_uri("http://localhost:9000/html")) ? |_| "send! /html failed"
    body = Str.from_utf8(Response.body(response)) ? |_| "HTML response was not UTF-8"

    write_line!("Response body:")?
    write_line!(body)?

    response_2 = Http.send!(Request.from_method(GET).with_uri("http://localhost:9000/html")) ? |_| "second send! /html failed"
    body_2 = Str.from_utf8(Response.body(response_2)) ? |_| "second HTML response was not UTF-8"

    write_line!("Response body 2:")?
    write_line!(body_2)?

    Ok({})
}

write_line! : Str => Try({}, Str)
write_line! = |message| {
    Stdout.line!(message) ? |_| "stdout write failed"
    Ok({})
}

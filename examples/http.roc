app [main!] {
	pf: platform "../platform/main.roc",
	http: "https://github.com/roc-lang/http/releases/download/1.0.0/6ZUwqYhCS8PU9Mo6MF7oV82ET2o7KYb57CLKDq4cq4sS.tar.zst",
}

import pf.OsStr exposing [OsStr]
import pf.Http
import pf.Stdout
import http.Request
import http.Response

# Demo of the basic-cli HTTP functions against a local server.

main! : List(OsStr) => Try({}, _)
main! = |_args| run_demo!()

run_demo! : () => Try({}, _)
run_demo! = || {
	hello_str = Http.get_utf8!("http://127.0.0.1:9000/utf8test") ? |err| GetUtf8Failed(err)
	write_line!("I received '${hello_str}' from the server.")?

	decoded : { foo : Str }
	decoded = Http.get!("http://127.0.0.1:9000") ? |err| GetJsonFailed(err)

	write_line!("The json I received was: { foo: \"${decoded.foo}\" }")?

	response = Http.send!(Request.from_method(GET).with_uri("http://127.0.0.1:9000/html")) ? |err| SendHtmlFailed(err)
	body = Str.from_utf8(Response.body(response)) ? |err| HtmlBodyUtf8Failed(err)

	write_line!("Response body:")?
	write_line!(body)?

	response_2 = Http.send!(Request.from_method(GET).with_uri("http://127.0.0.1:9000/html")) ? |err| SendSecondHtmlFailed(err)
	body_2 = Str.from_utf8(Response.body(response_2)) ? |err| SecondHtmlBodyUtf8Failed(err)

	write_line!("Response body 2:")?
	write_line!(body_2)?

	Ok({})
}

write_line! : Str => Try({}, _)
write_line! = |message| Stdout.line!(message)

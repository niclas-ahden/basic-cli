import Host
import InternalHttp
import http.Request
import http.Response

## Send requests using the shared
## [`roc-lang/http`](https://github.com/roc-lang/http) `Request` and `Response`
## types. This module supplies effects and small JSON/UTF-8 conveniences while
## leaving pure request and response construction to that package.
Http := [].{
    ## Errors raised by the host while sending a request, before a real HTTP
    ## response is available.
    TransportErr : InternalHttp.TransportErr

    ## Send an HTTP request, succeeding with a `Response.Response` or failing with an
    ## `HttpErr`.
    ##
    ## ```roc
    ## request = Request.from_method(GET).with_uri("https://www.roc-lang.org")
    ## response = Http.send!(request)?
    ## ```
    send! : Request.Request => Try(Response.Response, [HttpErr(TransportErr), ..])
    send! = |request| {
        host_response = Host.http_send_request!(InternalHttp.to_host_request(request)) ? HttpErr

        Ok(InternalHttp.from_host_response(host_response))
    }

    ## Encode a value as JSON and set it as the request body.
    ##
    ## This uses Roc's builtin JSON encoder, so the value's type determines the
    ## encoder through static dispatch.
    with_json_body : Request.Request, _ => Try(Request.Request, [JsonErr(_), ..])
    with_json_body = |request, value| {
        body = Json.to_str_try(value) ? JsonErr

        Ok(
            request
                .add_header("Content-Type", "application/json")
                .with_body(Str.to_utf8(body)),
        )
    }

    ## Encode a value as JSON, attach it to the request body, and send it.
    send_json! : Request.Request, _ => Try(Response.Response, [JsonErr(_), HttpErr(TransportErr), ..])
    send_json! = |request, value| {
        json_request = with_json_body(request, value)?

        send!(json_request)
    }

    ## Perform an HTTP GET and decode the response body as a UTF-8 `Str`.
    ##
    ## ```roc
    ## hello_str = Http.get_utf8!("http://localhost:8000")?
    ## ```
    get_utf8! : Str => Try(Str, [BadBody(Str), HttpErr(TransportErr), ..])
    get_utf8! = |uri| {
        response = send!(Request.from_method(GET).with_uri(uri))?
        body = Str.from_utf8(Response.body(response)) ? |_| BadBody("get_utf8!: response body was not valid UTF-8")

        Ok(body)
    }

    ## Decode a response body as JSON.
    ##
    ## This uses Roc's builtin JSON parser, so the expected result type
    ## determines the parser through static dispatch.
    decode_json_response : Response.Response => Try(_, [BadBody(Str), JsonErr(_), ..])
    decode_json_response = |response| {
        body = Str.from_utf8(Response.body(response)) ? |_| BadBody("decode_json_response: response body was not valid UTF-8")
        decoded = Json.parse(body) ? JsonErr

        Ok(decoded)
    }

    ## Perform an HTTP GET and decode the response body as JSON.
    ##
    ## JSON parser failures are returned as `JsonErr(_)`.
    ##
    ## ```roc
    ## payload : Try({ foo : Str }, _)
    ## payload = Http.get!("http://localhost:8000")
    ## ```
    get! : Str => Try(_, [BadBody(Str), HttpErr(TransportErr), JsonErr(_), ..])
    get! = |uri| {
        response = send!(Request.from_method(GET).with_uri(uri))?

        decode_json_response(response)
    }
}

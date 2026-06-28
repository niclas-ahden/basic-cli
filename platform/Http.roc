import http.Request
import http.Response

Http := [].{
    # The host-facing shapes flatten the request/response into a record of
    # primitives/lists/tuples so the generated glue stays simple. The HTTP method
    # is flattened into a compact `U8` tag (a known method costs one byte at the
    # boundary rather than a string), with `method_ext` carrying the method name
    # only for the `Unknown` case (empty otherwise). `timeout_ms` is a plain `U64`
    # with 0 meaning "no timeout". Headers are `(name, value)` tuples, matching
    # the roc-lang/http package representation.
    RequestToAndFromHost : {
        method : U8,
        method_ext : Str,
        headers : List((Str, Str)),
        uri : Str,
        body : List(U8),
        timeout_ms : U64,
    }

    ResponseToAndFromHost : {
        status : U16,
        headers : List((Str, Str)),
        body : List(U8),
    }

    # The single host effect: hand a fully-marshalled request to the host and get
    # back a marshalled response. Transport failures are encoded by the host as a
    # sentinel status+body pair (see `send!`), not as a separate error channel.
    host_send_request! : RequestToAndFromHost => ResponseToAndFromHost

    ## Send an HTTP request, succeeding with a [Response] or failing with an
    ## `HttpErr`.
    ##
    ## ```roc
    ## request = Request.from_method(GET) |> Request.with_uri("https://www.roc-lang.org")
    ## response = Http.send!(request)?
    ## ```
    send! : Request.Request => Try(Response.Response, [HttpErr([Timeout, NetworkError, BadBody, Other(List(U8))])])
    send! = |request| {
        host_response = Http.host_send_request!(to_host_request(request))

        # The host signals transport failures with these reserved status+body
        # sentinels (produced in src/lib.rs); everything else is a real response.
        other_error_prefix = Str.to_utf8("OTHER ERROR\n")

        if host_response.status == 408 and host_response.body == Str.to_utf8("Timeout") {
            Err(HttpErr(Timeout))
        } else if host_response.status == 500 and host_response.body == Str.to_utf8("NetworkError") {
            Err(HttpErr(NetworkError))
        } else if host_response.status == 500 and host_response.body == Str.to_utf8("BadBody") {
            Err(HttpErr(BadBody))
        } else if host_response.status == 500 and List.starts_with(host_response.body, other_error_prefix) {
            Err(HttpErr(Other(List.drop_first(host_response.body, List.len(other_error_prefix)))))
        } else {
            Ok(from_host_response(host_response))
        }
    }

    ## Perform an HTTP GET and decode the response body as a UTF-8 [Str].
    ##
    ## ```roc
    ## hello_str = Http.get_utf8!("http://localhost:8000")?
    ## ```
    get_utf8! : Str => Try(Str, [BadBody(Str), HttpErr([Timeout, NetworkError, BadBody, Other(List(U8))])])
    get_utf8! = |uri|
        match send!(Request.with_uri(Request.from_method(GET), uri)) {
            Err(HttpErr(err)) => Err(HttpErr(err))
            Ok(response) =>
                match Str.from_utf8(Response.body(response)) {
                    Ok(str) => Ok(str)
                    Err(_) => Err(BadBody("get_utf8!: response body was not valid UTF-8"))
                }
        }

    ## Perform an HTTP GET and decode the response body as JSON.
    ##
    ## JSON parser failures are returned as `JsonErr(Json)`.
    ##
    ## ```roc
    ## payload : Try({ foo : Str }, _)
    ## payload = Http.get!("http://localhost:8000")
    ## ```
    get! = |uri|
        match get_utf8!(uri) {
            Err(BadBody(err)) => Err(BadBody(err))
            Err(HttpErr(err)) => Err(HttpErr(err))
            Ok(body) =>
                match Json.parse(body) {
                    Ok(value) => Ok(value)
                    Err(err) => Err(JsonErr(err))
                }
        }
}

# ---- internal conversion helpers (module-private) ------------------------------

# Read the package's opaque `Request` via its accessors and flatten it into the
# host-facing record. The method is flattened to a numeric tag (+ extension
# string for `Unknown`).
to_host_request = |request| {
    method = Request.method(request)
    {
        method: to_host_method(method),
        method_ext: to_host_method_ext(method),
        headers: Request.headers(request),
        uri: Request.uri(request),
        body: Request.body(request),
        timeout_ms: to_host_timeout(Request.timeout(request)),
    }
}

# These numeric method tags must match `as_hyper_method` in src/lib.rs.
to_host_method = |method|
    match method {
        OPTIONS => 5
        GET => 3
        POST => 7
        PUT => 8
        DELETE => 1
        HEAD => 4
        TRACE => 9
        CONNECT => 0
        PATCH => 6
        Unknown(_) => 2
    }

to_host_method_ext = |method|
    match method {
        Unknown(ext) => ext
        _ => ""
    }

# Rebuild the package's opaque `Response` from the host-facing record using its
# constructors/builders (the fields are not directly accessible across packages).
from_host_response = |response| {
    r0 = Response.from_status(response.status)
    r1 = Response.with_headers(r0, response.headers)
    Response.with_body(r1, response.body)
}

to_host_timeout = |timeout|
    match timeout {
        TimeoutMilliseconds(ms) => ms
        NoTimeout => 0
    }

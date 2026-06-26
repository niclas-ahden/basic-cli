import InternalHttp

Http := [].{
    ## Represents an HTTP method.
    Method : InternalHttp.Method

    ## Represents an HTTP header e.g. `Content-Type: application/json`, as a
    ## `{ name : Str, value : Str }` record.
    Header : InternalHttp.Header

    ## Represents an HTTP request.
    Request : InternalHttp.Request

    ## Represents an HTTP response.
    Response : InternalHttp.Response

    # The single host effect: hand a fully-marshalled request to the host and get
    # back a marshalled response. Transport failures are encoded by the host as a
    # sentinel status+body pair (see `send!`), not as a separate error channel.
    host_send_request! : InternalHttp.RequestToAndFromHost => InternalHttp.ResponseToAndFromHost

    ## A default [Request]: `GET` with no headers, empty uri/body, and no timeout.
    ##
    ## ```roc
    ## { Http.default_request & uri: "https://www.roc-lang.org" }
    ## ```
    default_request : Request
    default_request = {
        method: GET,
        headers: [],
        uri: "",
        body: [],
        timeout_ms: NoTimeout,
    }

    ## Build an HTTP [Header] from a `(name, value)` tuple.
    ##
    ## ```roc
    ## Http.header(("Content-Type", "application/json"))
    ## ```
    header : (Str, Str) -> Header
    header = |(name, value)| { name, value }

    ## Send an HTTP request, succeeding with a [Response] or failing with an
    ## `HttpErr`.
    ##
    ## ```roc
    ## response = Http.send!({ Http.default_request & uri: "https://www.roc-lang.org" })?
    ## ```
    send! : Request => Try(Response, [HttpErr([Timeout, NetworkError, BadBody, Other(List(U8))])])
    send! = |request| {
        host_request = to_host_request(request)
        response = from_host_response(Http.host_send_request!(host_request))

        # The host signals transport failures with these reserved status+body
        # sentinels (produced in src/lib.rs); everything else is a real response.
        other_error_prefix = Str.to_utf8("OTHER ERROR\n")

        if response.status == 408 and response.body == Str.to_utf8("Timeout") {
            Err(HttpErr(Timeout))
        } else if response.status == 500 and response.body == Str.to_utf8("NetworkError") {
            Err(HttpErr(NetworkError))
        } else if response.status == 500 and response.body == Str.to_utf8("BadBody") {
            Err(HttpErr(BadBody))
        } else if response.status == 500 and List.starts_with(response.body, other_error_prefix) {
            Err(HttpErr(Other(List.drop_first(response.body, List.len(other_error_prefix)))))
        } else {
            Ok(response)
        }
    }

    ## Perform an HTTP GET and decode the response body as a UTF-8 [Str].
    ##
    ## ```roc
    ## hello_str = Http.get_utf8!("http://localhost:8000")?
    ## ```
    get_utf8! : Str => Try(Str, [BadBody(Str), HttpErr([Timeout, NetworkError, BadBody, Other(List(U8))])])
    get_utf8! = |uri|
        match send!({ method: GET, headers: [], uri: uri, body: [], timeout_ms: NoTimeout }) {
            Err(HttpErr(err)) => Err(HttpErr(err))
            Ok(response) =>
                match Str.from_utf8(response.body) {
                    Ok(str) => Ok(str)
                    Err(_) => Err(BadBody("get_utf8!: response body was not valid UTF-8"))
                }
        }
}

# ---- internal conversion helpers (module-private) ------------------------------

# These numeric method tags must match `as_hyper_method` in src/lib.rs.
to_host_request = |request| {
    method: to_host_method(request.method),
    method_ext: to_host_method_ext(request.method),
    headers: request.headers,
    uri: request.uri,
    body: request.body,
    timeout_ms: to_host_timeout(request.timeout_ms),
}

from_host_response = |response| {
    status: response.status,
    headers: response.headers,
    body: response.body,
}

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
        EXTENSION(_) => 2
    }

to_host_method_ext = |method|
    match method {
        EXTENSION(ext) => ext
        _ => ""
    }

to_host_timeout = |timeout|
    match timeout {
        TimeoutMilliseconds(ms) => ms
        NoTimeout => 0
    }

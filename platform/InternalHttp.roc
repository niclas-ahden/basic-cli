InternalHttp := [].{
    # https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods
    Method : [OPTIONS, GET, POST, PUT, DELETE, HEAD, TRACE, CONNECT, PATCH, EXTENSION(Str)]

    # https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers
    Header : { name : Str, value : Str }

    Request : {
        method : Method,
        headers : List(Header),
        uri : Str,
        body : List(U8),
        timeout_ms : [TimeoutMilliseconds(U64), NoTimeout],
    }

    Response : {
        status : U16,
        headers : List(Header),
        body : List(U8),
    }

    # The host-facing shapes flatten `Method` into a numeric tag (+ extension
    # string) and `timeout_ms` into a plain `U64` (0 meaning "no timeout"), so
    # the generated glue stays a simple record of primitives/lists.
    RequestToAndFromHost : {
        method : U64,
        method_ext : Str,
        headers : List(Header),
        uri : Str,
        body : List(U8),
        timeout_ms : U64,
    }

    ResponseToAndFromHost : {
        status : U16,
        headers : List(Header),
        body : List(U8),
    }
}

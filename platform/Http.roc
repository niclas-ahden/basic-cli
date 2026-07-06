import Host
import InternalHttp
import http.Request
import http.Response

Http := [].{
    ## Errors raised by the host while sending a request, before a real HTTP
    ## response is available.
    TransportErr : InternalHttp.TransportErr

    ## Send an HTTP request, succeeding with a [Response] or failing with an
    ## `HttpErr`.
    ##
    ## ```roc
    ## request = Request.from_method(GET) |> Request.with_uri("https://www.roc-lang.org")
    ## response = Http.send!(request)?
    ## ```
    send! : Request.Request => Try(Response.Response, [HttpErr(TransportErr), ..])
    send! = |request| {
        host_response = Host.http_send_request!(InternalHttp.to_host_request(request)) ? HttpErr

        Ok(InternalHttp.from_host_response(host_response))
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
        body = get_utf8!(uri)?
        decoded = Json.parse(body) ? JsonErr

        Ok(decoded)
    }
}

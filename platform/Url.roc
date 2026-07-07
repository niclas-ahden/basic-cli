Url := [].{
    from_str : Str -> Str
    from_str = |str| str

    to_str : Str -> Str
    to_str = |url| url

    reserve : Str, U64 -> Str
    reserve = |url, _capacity| url

    append : Str, Str -> Str
    append = |url, suffix_unencoded| {
        suffix_parts = Str.split_on(suffix_unencoded, "/")
        suffix = Str.join_with(List.map(suffix_parts, percent_encode), "/")

        match split_first(url, "?") {
            Found({ before, after }) => "${append_path(before, suffix)}?${after}"
            NotFound =>
                match split_first(url, "#") {
                    Found({ before, after }) => "${append_path(before, suffix)}#${after}"
                    NotFound => append_path(url, suffix)
                }
        }
    }

    append_param : Str, Str, Str -> Str
    append_param = |url, key, value| {
        { without_fragment, fragment_suffix } =
            match split_last(url, "#") {
                Found({ before, after }) => { without_fragment: before, fragment_suffix: "#${after}" }
                NotFound => { without_fragment: url, fragment_suffix: "" }
            }

        separator = if has_query(without_fragment) { "&" } else { "?" }

        "${without_fragment}${separator}${percent_encode(key)}=${percent_encode(value)}${fragment_suffix}"
    }

    has_query : Str -> Bool
    has_query = |url| Str.contains(url, "?")

    has_fragment : Str -> Bool
    has_fragment = |url| Str.contains(url, "#")

    query : Str -> Str
    query = |url| {
        without_fragment =
            match split_last(url, "#") {
                Found({ before, after: _ }) => before
                NotFound => url
            }

        match split_last(without_fragment, "?") {
            Found({ before: _, after }) => after
            NotFound => ""
        }
    }

    fragment : Str -> Str
    fragment = |url|
        match split_last(url, "#") {
            Found({ before: _, after }) => after
            NotFound => ""
        }

    with_query : Str, Str -> Str
    with_query = |url, query_str| {
        { without_fragment, fragment_suffix } =
            match split_last(url, "#") {
                Found({ before, after }) => { without_fragment: before, fragment_suffix: "#${after}" }
                NotFound => { without_fragment: url, fragment_suffix: "" }
            }

        before_query =
            match split_last(without_fragment, "?") {
                Found({ before, after: _ }) => before
                NotFound => without_fragment
            }

        if Str.is_empty(query_str) {
            "${before_query}${fragment_suffix}"
        } else {
            "${before_query}?${query_str}${fragment_suffix}"
        }
    }

    with_fragment : Str, Str -> Str
    with_fragment = |url, fragment_str|
        match split_last(url, "#") {
            Found({ before, after: _ }) =>
                if Str.is_empty(fragment_str) {
                    before
                } else {
                    "${before}#${fragment_str}"
                }
            NotFound =>
                if Str.is_empty(fragment_str) {
                    url
                } else {
                    "${url}#${fragment_str}"
                }
        }

    query_params : Str -> Dict(Str, Str)
    query_params = |url| {
        pairs = Str.split_on(query(url), "&")

        Iter.fold(List.iter(pairs), Dict.empty(), |dict, pair|
            match split_first(pair, "=") {
                Found({ before, after }) => Dict.insert(dict, before, after)
                NotFound => Dict.insert(dict, pair, "")
            })
    }

    path : Str -> Str
    path = |url| {
        without_authority =
            if starts_with(url, "/") {
                url
            } else {
                match split_first(url, ":") {
                    Found({ before: _, after }) =>
                        match split_first(after, "//") {
                            Found({ before, after: after_slashes }) =>
                                if Str.is_empty(before) {
                                    after_slashes
                                } else {
                                    after
                                }
                            NotFound => after
                        }
                    NotFound => url
                }
            }

        without_query =
            match split_last(without_authority, "?") {
                Found({ before, after: _ }) => before
                NotFound => without_authority
            }

        match split_last(without_query, "#") {
            Found({ before, after: _ }) => before
            NotFound => without_query
        }
    }
}

append_path : Str, Str -> Str
append_path = |prefix, suffix| {
    if Str.is_empty(prefix) {
        suffix
    } else if Str.is_empty(suffix) {
        prefix
    } else if ends_with(prefix, "/") {
        if starts_with(suffix, "/") {
            "${prefix}${drop_first_part(suffix, "/")}"
        } else {
            "${prefix}${suffix}"
        }
    } else if starts_with(suffix, "/") {
        "${prefix}${suffix}"
    } else {
        "${prefix}/${suffix}"
    }
}

percent_encode : Str -> Str
percent_encode = |input| {
    encoded_parts =
        List.map(Str.to_utf8(input), |byte|
            if is_unreserved(byte) {
                byte_to_str(byte)
            } else {
                encode_byte(byte)
            })

    Str.join_with(encoded_parts, "")
}

is_unreserved : U8 -> Bool
is_unreserved = |byte| {
    if byte >= 97 {
        if byte <= 122 {
            True
        } else {
            is_unreserved_symbol(byte)
        }
    } else if byte >= 65 {
        if byte <= 90 {
            True
        } else {
            is_unreserved_symbol(byte)
        }
    } else if byte >= 48 {
        if byte <= 57 {
            True
        } else {
            is_unreserved_symbol(byte)
        }
    } else {
        is_unreserved_symbol(byte)
    }
}

is_unreserved_symbol : U8 -> Bool
is_unreserved_symbol = |byte|
    match byte {
        45 => True
        46 => True
        95 => True
        126 => True
        _ => False
    }

byte_to_str : U8 -> Str
byte_to_str = |byte|
    match Str.from_utf8([byte]) {
        Ok(str) => str
        Err(_) => ""
    }

encode_byte : U8 -> Str
encode_byte = |byte| {
    high = byte // 16
    low = byte % 16

    "%${hex_digit(high)}${hex_digit(low)}"
}

hex_digit : U8 -> Str
hex_digit = |value|
    match value {
        0 => "0"
        1 => "1"
        2 => "2"
        3 => "3"
        4 => "4"
        5 => "5"
        6 => "6"
        7 => "7"
        8 => "8"
        9 => "9"
        10 => "A"
        11 => "B"
        12 => "C"
        13 => "D"
        14 => "E"
        _ => "F"
    }

starts_with : Str, Str -> Bool
starts_with = |str, prefix|
    if Str.is_empty(prefix) {
        True
    } else {
        parts = Str.split_on(str, prefix)

        match parts.get(0) {
            Ok(first) => Str.is_empty(first)
            Err(_) => False
        }
    }

ends_with : Str, Str -> Bool
ends_with = |str, suffix|
    if Str.is_empty(suffix) {
        True
    } else {
        parts = Str.split_on(str, suffix)
        last_index = List.len(parts) - 1

        match parts.get(last_index) {
            Ok(last) => Str.is_empty(last)
            Err(_) => False
        }
    }

drop_first_part : Str, Str -> Str
drop_first_part = |str, separator| {
    parts = Str.split_on(str, separator)
    rest = List.drop_first(parts, 1)

    Str.join_with(rest, separator)
}

split_first : Str, Str -> [Found({ after : Str, before : Str }), NotFound]
split_first = |str, separator| {
    parts = Str.split_on(str, separator)

    if List.len(parts) > 1 {
        match parts.get(0) {
            Ok(before) => {
                after = Str.join_with(List.drop_first(parts, 1), separator)
                Found({ before, after })
            }
            Err(_) => NotFound
        }
    } else {
        NotFound
    }
}

split_last : Str, Str -> [Found({ after : Str, before : Str }), NotFound]
split_last = |str, separator| {
    parts = Str.split_on(str, separator)

    if List.len(parts) > 1 {
        last_index = List.len(parts) - 1
        before = Str.join_with(List.drop_last(parts, 1), separator)

        match parts.get(last_index) {
            Ok(after) => Found({ before, after })
            Err(_) => NotFound
        }
    } else {
        NotFound
    }
}

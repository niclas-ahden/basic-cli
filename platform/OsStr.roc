import path.Path as PackagePath

OsStr := [
    Utf8(Str),
    UnixBytes(List(U8)),
    WindowsU16s(List(U16)),
].{
    ## Create an OS string from UTF-8 text.
    ## The host lowers this text to the active OS representation.
    from_str : Str -> OsStr
    from_str = |str| Utf8(str)

    ## Create a UTF-8 text OS string.
    utf8 : Str -> OsStr
    utf8 = |str| Utf8(str)

    ## Create a Unix OS string from a Roc string by storing its UTF-8 bytes.
    unix : Str -> OsStr
    unix = |str| UnixBytes(Str.to_utf8(str))

    ## Create a Unix OS string from raw bytes without validating UTF-8.
    unix_bytes : List(U8) -> OsStr
    unix_bytes = |bytes| UnixBytes(bytes)

    ## Create a Windows OS string from a Roc string by storing its UTF-16 code units.
    windows : Str -> OsStr
    windows = |str| from_path(PackagePath.windows(str))

    ## Create a Windows OS string from raw UTF-16 code units.
    windows_u16s : List(U16) -> OsStr
    windows_u16s = |u16s| WindowsU16s(u16s)

    ## Build an OS string from a quoted string literal.
    from_quote : Str -> Try(OsStr, [BadQuotedBytes(Str)])
    from_quote = |str| Ok(Utf8(str))

    ## Convert an OS string to a string if its raw representation is valid text.
    to_str_try : OsStr -> Try(Str, [InvalidStr(U64)])
    to_str_try = |os_str| PackagePath.to_str(to_path(os_str))

    ## Convert an OS string to a display string, replacing invalid text with U+FFFD.
    display : OsStr -> Str
    display = |os_str| PackagePath.display(to_path(os_str))

    ## Customize debug output for `Str.inspect`.
    to_inspect : OsStr -> Str
    to_inspect = |os_str| "OsStr(${display(os_str)})"

    ## Expose the host ABI representation.
    to_raw : OsStr -> [Utf8(Str), UnixBytes(List(U8)), WindowsU16s(List(U16))]
    to_raw = |os_str|
        match os_str {
            Utf8(str) => Utf8(str)
            UnixBytes(bytes) => UnixBytes(bytes)
            WindowsU16s(u16s) => WindowsU16s(u16s)
        }

    ## Build an OS string from the host ABI representation.
    from_raw : [Utf8(Str), UnixBytes(List(U8)), WindowsU16s(List(U16))] -> OsStr
    from_raw = |raw|
        match raw {
            Utf8(str) => Utf8(str)
            UnixBytes(bytes) => UnixBytes(bytes)
            WindowsU16s(u16s) => WindowsU16s(u16s)
        }

    ## Interpret this OS string as a path without changing its representation.
    to_path : OsStr -> PackagePath.Path
    to_path = |os_str|
        PackagePath.from_raw(to_raw(os_str))

    ## Build an OS string from a path without changing its representation.
    from_path : PackagePath.Path -> OsStr
    from_path = |path|
        from_raw(PackagePath.to_raw(path))
}

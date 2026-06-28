import IOErr exposing [IOErr]

File := [].{
    ## Represents a buffered file reader.
    ##
    ## The file is automatically closed when the last reference to the reader is
    ## dropped. This is an opaque `Box(U64)` handle into a host-side
    ## `BufReader<File>`.
    Reader :: Box(U64)

    # ---- Host functions (the FFI boundary) -------------------------------------
    host_open_reader! : Str, U64 => Try(Reader, [FileErr(IOErr), ..])
    host_read_line! : Reader => Try(List(U8), [FileErr(IOErr), ..])

    ## Read all bytes from a file.
    read_bytes! : Str => Try(List(U8), [FileErr(IOErr), ..])

    ## Write bytes to a file, replacing any existing contents.
    write_bytes! : Str, List(U8) => Try({}, [FileErr(IOErr), ..])

    ## Read a file's contents as a UTF-8 string.
    ##
    ## If the file contains invalid UTF-8, the invalid parts will be replaced with the
    ## [Unicode replacement character](https://unicode.org/glossary/#replacement_character).
    read_utf8! : Str => Try(Str, [FileErr(IOErr), ..])

    ## Write a UTF-8 string to a file, replacing any existing contents.
    write_utf8! : Str, Str => Try({}, [FileErr(IOErr), ..])

    ## Open a file for buffered reading using the default buffer capacity.
    ##
    ## ```roc
    ## reader = File.open_reader!("LICENSE")?
    ## line = File.read_line!(reader)?
    ## ```
    open_reader! = |path|
        File.host_open_reader!(path, 0)

    ## Open a file for buffered reading using a specific buffer capacity.
    open_reader_with_capacity! = |path, capacity|
        File.host_open_reader!(path, capacity)

    ## Read bytes up to and including the next newline from a buffered reader.
    ##
    ## Returns an empty list at EOF.
    read_line! = |reader|
        File.host_read_line!(reader)

    ## Delete a file.
    delete! : Str => Try({}, [FileErr(IOErr), ..])

    ## Returns the size of a file in bytes.
    size_in_bytes! : Str => Try(U64, [FileErr(IOErr), ..])

    ## Checks if the file has any executable bit set.
    is_executable! : Str => Try(Bool, [FileErr(IOErr), ..])

    ## Checks if the file has a readable owner permission bit set.
    is_readable! : Str => Try(Bool, [FileErr(IOErr), ..])

    ## Checks if the file has a writable owner permission bit set.
    is_writable! : Str => Try(Bool, [FileErr(IOErr), ..])

    ## Returns the time when the file was last accessed as nanoseconds since the Unix epoch.
    time_accessed! : Str => Try(U128, [FileErr(IOErr), ..])

    ## Returns the time when the file was last modified as nanoseconds since the Unix epoch.
    time_modified! : Str => Try(U128, [FileErr(IOErr), ..])

    ## Returns the time when the file was created as nanoseconds since the Unix epoch.
    time_created! : Str => Try(U128, [FileErr(IOErr), ..])
}

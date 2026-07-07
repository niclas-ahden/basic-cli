import IOErr exposing [IOErr]
import Host

File := [].{
    ## Represents a buffered file reader.
    ##
    ## The file is automatically closed when the last reference to the reader is
    ## dropped. This is an opaque `Box(U64)` handle into a host-side
    ## `BufReader<File>`.
    Reader : Host.FileReader

    ## Read all bytes from a file.
    read_bytes! : Str => Try(List(U8), [FileErr(IOErr), ..])
    read_bytes! = |path| widen_file_err(Host.file_read_bytes!(path))

    ## Write bytes to a file, replacing any existing contents.
    write_bytes! : Str, List(U8) => Try({}, [FileErr(IOErr), ..])
    write_bytes! = |path, bytes| widen_file_err(Host.file_write_bytes!(path, bytes))

    ## Read a file's contents as a UTF-8 string.
    ##
    ## If the file contains invalid UTF-8, the invalid parts will be replaced with the
    ## [Unicode replacement character](https://unicode.org/glossary/#replacement_character).
    read_utf8! : Str => Try(Str, [FileErr(IOErr), ..])
    read_utf8! = |path| widen_file_err(Host.file_read_utf8!(path))

    ## Write a UTF-8 string to a file, replacing any existing contents.
    write_utf8! : Str, Str => Try({}, [FileErr(IOErr), ..])
    write_utf8! = |path, content| widen_file_err(Host.file_write_utf8!(path, content))

    ## Open a file for buffered reading using the default buffer capacity.
    ##
    ## ```roc
    ## reader = File.open_reader!("LICENSE")?
    ## line = File.read_line!(reader)?
    ## ```
    open_reader! = |path|
        widen_file_err(Host.file_open_reader!(path, 0))

    ## Open a file for buffered reading using a specific buffer capacity.
    open_reader_with_capacity! = |path, capacity|
        widen_file_err(Host.file_open_reader!(path, capacity))

    ## Read bytes up to and including the next newline from a buffered reader.
    ##
    ## Returns an empty list at EOF.
    read_line! = |reader|
        widen_file_err(Host.file_read_line!(reader))

    ## Delete a file.
    delete! : Str => Try({}, [FileErr(IOErr), ..])
    delete! = |path| widen_file_err(Host.file_delete!(path))

    ## Returns the size of a file in bytes.
    size_in_bytes! : Str => Try(U64, [FileErr(IOErr), ..])
    size_in_bytes! = |path| widen_file_err(Host.file_size_in_bytes!(path))

    ## Checks if the file has any executable bit set.
    is_executable! : Str => Try(Bool, [FileErr(IOErr), ..])
    is_executable! = |path| widen_file_err(Host.file_is_executable!(path))

    ## Checks if the file has a readable owner permission bit set.
    is_readable! : Str => Try(Bool, [FileErr(IOErr), ..])
    is_readable! = |path| widen_file_err(Host.file_is_readable!(path))

    ## Checks if the file has a writable owner permission bit set.
    is_writable! : Str => Try(Bool, [FileErr(IOErr), ..])
    is_writable! = |path| widen_file_err(Host.file_is_writable!(path))

    ## Returns the time when the file was last accessed as nanoseconds since the Unix epoch.
    time_accessed! : Str => Try(U128, [FileErr(IOErr), ..])
    time_accessed! = |path| widen_file_err(Host.file_time_accessed!(path))

    ## Returns the time when the file was last modified as nanoseconds since the Unix epoch.
    time_modified! : Str => Try(U128, [FileErr(IOErr), ..])
    time_modified! = |path| widen_file_err(Host.file_time_modified!(path))

    ## Returns the time when the file was created as nanoseconds since the Unix epoch.
    time_created! : Str => Try(U128, [FileErr(IOErr), ..])
    time_created! = |path| widen_file_err(Host.file_time_created!(path))
}

widen_file_err : Try(a, [FileErr(IOErr)]) -> Try(a, [FileErr(IOErr), ..])
widen_file_err = |result|
    match result {
        Ok(value) => Ok(value),
        Err(FileErr(err)) => Err(FileErr(err)),
    }

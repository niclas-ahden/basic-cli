import IOErr exposing [IOErr]
import Host
import path.Path as PackagePath

File := [].{
    ## Represents a buffered file reader.
    ##
    ## The file is automatically closed when the last reference to the reader is
    ## dropped. This is an opaque `Box(U64)` handle into a host-side
    ## `BufReader<File>`.
    Reader : Host.FileReader

    ## Read all bytes from a file.
    read_bytes! : PackagePath.Path => Try(List(U8), [FileErr(IOErr), ..])
    read_bytes! = |path| widen_file_err(Host.file_read_bytes!(PackagePath.to_raw(path)))

    ## Write bytes to a file, replacing any existing contents.
    write_bytes! : PackagePath.Path, List(U8) => Try({}, [FileErr(IOErr), ..])
    write_bytes! = |path, bytes| widen_file_err(Host.file_write_bytes!(PackagePath.to_raw(path), bytes))

    ## Read a file's contents as a UTF-8 string.
    ##
    ## If the file contains invalid UTF-8, the invalid parts will be replaced with the
    ## [Unicode replacement character](https://unicode.org/glossary/#replacement_character).
    read_utf8! : PackagePath.Path => Try(Str, [FileErr(IOErr), ..])
    read_utf8! = |path| widen_file_err(Host.file_read_utf8!(PackagePath.to_raw(path)))

    ## Write a UTF-8 string to a file, replacing any existing contents.
    write_utf8! : PackagePath.Path, Str => Try({}, [FileErr(IOErr), ..])
    write_utf8! = |path, content| widen_file_err(Host.file_write_utf8!(PackagePath.to_raw(path), content))

    ## Create a hard link at `link` pointing to `original`.
    hard_link! : PackagePath.Path, PackagePath.Path => Try({}, [FileErr(IOErr), ..])
    hard_link! = |original, link| widen_file_err(Host.file_hard_link!(PackagePath.to_raw(original), PackagePath.to_raw(link)))

    ## Rename a file from `from` to `to`.
    rename! : PackagePath.Path, PackagePath.Path => Try({}, [FileErr(IOErr), ..])
    rename! = |from, to| widen_file_err(Host.file_rename!(PackagePath.to_raw(from), PackagePath.to_raw(to)))

    ## Open a file for buffered reading using the default buffer capacity.
    ##
    ## ```roc
    ## reader = File.open_reader!("LICENSE")?
    ## line = File.read_line!(reader)?
    ## ```
    open_reader! = |path|
        widen_file_err(Host.file_open_reader!(PackagePath.to_raw(path), 0))

    ## Open a file for buffered reading using a specific buffer capacity.
    open_reader_with_capacity! = |path, capacity|
        widen_file_err(Host.file_open_reader!(PackagePath.to_raw(path), capacity))

    ## Read bytes up to and including the next newline from a buffered reader.
    ##
    ## Returns an empty list at EOF.
    read_line! = |reader|
        widen_file_err(Host.file_read_line!(reader))

    ## Delete a file.
    delete! : PackagePath.Path => Try({}, [FileErr(IOErr), ..])
    delete! = |path| widen_file_err(Host.file_delete!(PackagePath.to_raw(path)))

    ## Returns `True` if the path exists on disk.
    exists! : PackagePath.Path => Try(Bool, [FileErr(IOErr), ..])
    exists! = |path|
        match type!(path) {
            Ok(_) => Ok(Bool.True),
            Err(FileErr(NotFound)) => Ok(Bool.False),
            Err(err) => Err(err),
        }

    ## Returns `True` if the path exists on disk and is a regular file.
    is_file! : PackagePath.Path => Try(Bool, [FileErr(IOErr), ..])
    is_file! = |path|
        match type!(path) {
            Ok(IsFile) => Ok(Bool.True),
            Ok(_) => Ok(Bool.False),
            Err(FileErr(NotFound)) => Ok(Bool.False),
            Err(err) => Err(err),
        }

    ## Returns `True` if the path exists on disk and is a symbolic link.
    is_sym_link! : PackagePath.Path => Try(Bool, [FileErr(IOErr), ..])
    is_sym_link! = |path|
        match type!(path) {
            Ok(IsSymLink) => Ok(Bool.True),
            Ok(_) => Ok(Bool.False),
            Err(FileErr(NotFound)) => Ok(Bool.False),
            Err(err) => Err(err),
        }

    ## Return the type of the path if it exists on disk.
    type! : PackagePath.Path => Try([IsFile, IsDir, IsSymLink], [FileErr(IOErr), ..])
    type! = |path|
        match Host.path_type!(PackagePath.to_raw(path)) {
            Ok(path_type) =>
                if path_type.is_sym_link {
                    Ok(IsSymLink)
                } else if path_type.is_dir {
                    Ok(IsDir)
                } else {
                    Ok(IsFile)
                }

            Err(err) => Err(FileErr(err)),
        }

    ## Returns the size of a file in bytes.
    size_in_bytes! : PackagePath.Path => Try(U64, [FileErr(IOErr), ..])
    size_in_bytes! = |path| widen_file_err(Host.file_size_in_bytes!(PackagePath.to_raw(path)))

    ## Checks if the file has any executable bit set.
    is_executable! : PackagePath.Path => Try(Bool, [FileErr(IOErr), ..])
    is_executable! = |path| widen_file_err(Host.file_is_executable!(PackagePath.to_raw(path)))

    ## Checks if the file has a readable owner permission bit set.
    is_readable! : PackagePath.Path => Try(Bool, [FileErr(IOErr), ..])
    is_readable! = |path| widen_file_err(Host.file_is_readable!(PackagePath.to_raw(path)))

    ## Checks if the file has a writable owner permission bit set.
    is_writable! : PackagePath.Path => Try(Bool, [FileErr(IOErr), ..])
    is_writable! = |path| widen_file_err(Host.file_is_writable!(PackagePath.to_raw(path)))

    ## Returns the time when the file was last accessed as nanoseconds since the Unix epoch.
    time_accessed! : PackagePath.Path => Try(U128, [FileErr(IOErr), ..])
    time_accessed! = |path| widen_file_err(Host.file_time_accessed!(PackagePath.to_raw(path)))

    ## Returns the time when the file was last modified as nanoseconds since the Unix epoch.
    time_modified! : PackagePath.Path => Try(U128, [FileErr(IOErr), ..])
    time_modified! = |path| widen_file_err(Host.file_time_modified!(PackagePath.to_raw(path)))

    ## Returns the time when the file was created as nanoseconds since the Unix epoch.
    time_created! : PackagePath.Path => Try(U128, [FileErr(IOErr), ..])
    time_created! = |path| widen_file_err(Host.file_time_created!(PackagePath.to_raw(path)))
}

widen_file_err : Try(a, [FileErr(IOErr)]) -> Try(a, [FileErr(IOErr), ..])
widen_file_err = |result|
    match result {
        Ok(value) => Ok(value),
        Err(FileErr(err)) => Err(FileErr(err)),
    }

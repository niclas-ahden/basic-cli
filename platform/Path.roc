import IOErr exposing [IOErr]
import Host
import OsStr exposing [OsStr]
import path.Path as PackagePath

## Construct and operate on the shared
## [`roc-lang/path`](https://github.com/roc-lang/path) `Path` type. Native Unix
## bytes and Windows UTF-16 units are preserved across host effects; use
## `display` only when a lossy human-readable representation is appropriate.
Path := [].{
    ## Create a UTF-8 text path.
    ## The host lowers this text to the active OS path representation.
    from_str : Str -> PackagePath.Path
    from_str = |str| PackagePath.utf8(str)

    ## Create a path from an OS string value, preserving raw OS units.
    from_os_str : OsStr -> PackagePath.Path
    from_os_str = |os_str| OsStr.to_path(os_str)

    ## Convert a path to an OS string value, preserving raw OS units.
    to_os_str : PackagePath.Path -> OsStr
    to_os_str = |path| OsStr.from_path(path)

    ## Create a Unix path from a Roc string by storing its UTF-8 bytes.
    unix : Str -> PackagePath.Path
    unix = |str| PackagePath.unix(str)

    ## Create a Unix path from raw bytes without validating UTF-8.
    unix_bytes : List(U8) -> PackagePath.Path
    unix_bytes = |bytes| PackagePath.unix_bytes(bytes)

    ## Create a path from raw bytes without validating UTF-8.
    from_bytes : List(U8) -> PackagePath.Path
    from_bytes = |bytes| PackagePath.unix_bytes(bytes)

    ## Create a Windows path from a Roc string by storing its UTF-16 code units.
    windows : Str -> PackagePath.Path
    windows = |str| PackagePath.windows(str)

    ## Create a Windows path from raw UTF-16 code units.
    windows_u16s : List(U16) -> PackagePath.Path
    windows_u16s = |u16s| PackagePath.windows_u16s(u16s)

    ## Convert a path to a string if its raw representation is valid text.
    to_str_try : PackagePath.Path -> Try(Str, [InvalidStr(U64)])
    to_str_try = |path| PackagePath.to_str(path)

    ## Convert a path to a string if its raw representation is valid text.
    to_str : PackagePath.Path -> Try(Str, [InvalidStr(U64)])
    to_str = |path| to_str_try(path)

    ## Convert a path to a display string, replacing invalid text with U+FFFD.
    display : PackagePath.Path -> Str
    display = |path| PackagePath.display(path)

    ## Returns everything after the last directory separator.
    filename : PackagePath.Path -> Try(PackagePath.Path, [IsDirPath, EndsInDots])
    filename = |path| PackagePath.filename(path)

    ## Returns the filename extension without the leading dot.
    ext : PackagePath.Path -> Try(PackagePath.Path, [IsDirPath, EndsInDots])
    ext = |path| PackagePath.ext(path)

    ## Adds a separator and a string component to the path.
    join : PackagePath.Path, Str -> PackagePath.Path
    join = |path, str| PackagePath.join(path, str)

    ## Add or replace the filename extension.
    with_extension : PackagePath.Path, Str -> PackagePath.Path
    with_extension = |path, new_ext| {
        path_str = display(path)

        parts = Str.split_on(path_str, ".")
        base =
            if List.len(parts) > 1 {
                Str.join_with(List.drop_last(parts, 1), ".")
            } else {
                path_str
            }

        from_str("${base}.${new_ext}")
    }

    ## Expose the host ABI representation.
    to_raw : PackagePath.Path -> [Utf8(Str), UnixBytes(List(U8)), WindowsU16s(List(U16))]
    to_raw = |path| PackagePath.to_raw(path)

    ## Build a path from the host ABI representation.
    from_raw : [Utf8(Str), UnixBytes(List(U8)), WindowsU16s(List(U16))] -> PackagePath.Path
    from_raw = |raw| PackagePath.from_raw(raw)

    ## Returns `Bool.True` if the path exists on disk and is pointing at a regular file.
    ##
    ## This function will traverse symbolic links to query information about the
    ## destination file. In case of broken symbolic links this will return `Bool.False`.
    is_file! : PackagePath.Path => Try(Bool, [PathErr(IOErr), ..])
    is_file! = |path|
        match type!(path) {
            Ok(IsFile) => Ok(Bool.True)
            Ok(_) => Ok(Bool.False)
            Err(PathErr(NotFound)) => Ok(Bool.False)
            Err(err) => Err(err)
        }

    ## Returns `Bool.True` if the path exists on disk and is pointing at a directory.
    ##
    ## This function will traverse symbolic links to query information about the
    ## destination file. In case of broken symbolic links this will return `Bool.False`.
    is_dir! : PackagePath.Path => Try(Bool, [PathErr(IOErr), ..])
    is_dir! = |path|
        match type!(path) {
            Ok(IsDir) => Ok(Bool.True)
            Ok(_) => Ok(Bool.False)
            Err(PathErr(NotFound)) => Ok(Bool.False)
            Err(err) => Err(err)
        }

    ## Returns `Bool.True` if the path exists on disk and is pointing at a symbolic link.
    ##
    ## This function will not traverse symbolic links - it checks whether the path
    ## itself is a symlink.
    is_sym_link! : PackagePath.Path => Try(Bool, [PathErr(IOErr), ..])
    is_sym_link! = |path|
        match type!(path) {
            Ok(IsSymLink) => Ok(Bool.True)
            Ok(_) => Ok(Bool.False)
            Err(PathErr(NotFound)) => Ok(Bool.False)
            Err(err) => Err(err)
        }

    ## Returns `True` if the path exists on disk.
    exists! : PackagePath.Path => Try(Bool, [PathErr(IOErr), ..])
    exists! = |path|
        match type!(path) {
            Ok(_) => Ok(Bool.True)
            Err(PathErr(NotFound)) => Ok(Bool.False)
            Err(err) => Err(err)
        }

    ## Return the type of the path if the path exists on disk.
    ##
    ## > [`File.type`](File#type!) does the same thing.
    type! : PackagePath.Path => Try([IsFile, IsDir, IsSymLink], [PathErr(IOErr), ..])
    type! = |path| {
        Host.path_type!(PackagePath.to_raw(path))
            .map_err(|err| PathErr(err))
            .map_ok(|path_type|{
                if path_type.is_sym_link {
                    IsSymLink
                } else if path_type.is_dir {
                    IsDir
                } else {
                    IsFile
                }
            })
    }

    ## Read all bytes from a file at this path.
    read_bytes! : PackagePath.Path => Try(List(U8), [PathErr(IOErr), ..])
    read_bytes! = |path| map_file_result(Host.file_read_bytes!(PackagePath.to_raw(path)))

    ## Write bytes to a file at this path, replacing any existing contents.
    write_bytes! : List(U8), PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    write_bytes! = |bytes, path| map_file_result(Host.file_write_bytes!(PackagePath.to_raw(path), bytes))

    ## Read a UTF-8 file at this path.
    read_utf8! : PackagePath.Path => Try(Str, [PathErr(IOErr), ..])
    read_utf8! = |path| map_file_result(Host.file_read_utf8!(PackagePath.to_raw(path)))

    ## Write a UTF-8 file at this path, replacing any existing contents.
    write_utf8! : Str, PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    write_utf8! = |content, path| map_file_result(Host.file_write_utf8!(PackagePath.to_raw(path), content))

    ## Delete a file at this path.
    delete! : PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    delete! = |path| map_file_result(Host.file_delete!(PackagePath.to_raw(path)))

    ## Create a hard link at `link` pointing to `original`.
    hard_link! : PackagePath.Path, PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    hard_link! = |original, link|
        map_file_result(Host.file_hard_link!(PackagePath.to_raw(original), PackagePath.to_raw(link)))

    ## Rename a file from `from` to `to`.
    rename! : PackagePath.Path, PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    rename! = |from, to|
        map_file_result(Host.file_rename!(PackagePath.to_raw(from), PackagePath.to_raw(to)))

    ## Create a directory at this path.
    create_dir! : PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    create_dir! = |path| map_dir_result(Host.dir_create!(PackagePath.to_raw(path)))

    ## Create a directory and any missing parent directories at this path.
    create_all! : PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    create_all! = |path| map_dir_result(Host.dir_create_all!(PackagePath.to_raw(path)))

    ## Delete an empty directory at this path.
    delete_empty! : PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    delete_empty! = |path| map_dir_result(Host.dir_delete_empty!(PackagePath.to_raw(path)))

    ## Delete a directory and all contents at this path.
    delete_all! : PackagePath.Path => Try({}, [PathErr(IOErr), ..])
    delete_all! = |path| map_dir_result(Host.dir_delete_all!(PackagePath.to_raw(path)))
}

map_file_result : Try(a, [FileErr(IOErr)]) -> Try(a, [PathErr(IOErr), ..])
map_file_result = |result|
    match result {
        Ok(value) => Ok(value)
        Err(FileErr(err)) => Err(PathErr(err))
    }

map_dir_result : Try(a, [DirErr(IOErr)]) -> Try(a, [PathErr(IOErr), ..])
map_dir_result = |result|
    match result {
        Ok(value) => Ok(value)
        Err(DirErr(err)) => Err(PathErr(err))
    }

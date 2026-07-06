import IOErr exposing [IOErr]
import Host
import path.Path as PackagePath

Path := [].{
    ## Create a path from a Roc string using the host platform's native path representation.
    ##
    ## basic-cli currently targets Unix-like hosts, so this stores the string's UTF-8 bytes
    ## as a Unix path.
    from_str : Str -> PackagePath.Path
    from_str = |str| PackagePath.unix(str)

    ## Create a Unix path from a Roc string by storing its UTF-8 bytes.
    unix : Str -> PackagePath.Path
    unix = |str| PackagePath.unix(str)

    ## Create a Unix path from raw bytes without validating UTF-8.
    unix_bytes : List(U8) -> PackagePath.Path
    unix_bytes = |bytes| PackagePath.unix_bytes(bytes)

    ## Create a Windows path from a Roc string by storing its UTF-16 code units.
    windows : Str -> PackagePath.Path
    windows = |str| PackagePath.windows(str)

    ## Create a Windows path from raw UTF-16 code units.
    windows_u16s : List(U16) -> PackagePath.Path
    windows_u16s = |u16s| PackagePath.windows_u16s(u16s)

    ## Convert a path to a string if its raw representation is valid text.
    to_str : PackagePath.Path -> Try(Str, [InvalidStr(U64)])
    to_str = |path| path.to_str()

    ## Convert a path to a display string, replacing invalid text with U+FFFD.
    display : PackagePath.Path -> Str
    display = |path| path.display()

    ## Returns everything after the last directory separator.
    filename : PackagePath.Path -> Try(PackagePath.Path, [IsDirPath, EndsInDots])
    filename = |path| path.filename()

    ## Returns the filename extension without the leading dot.
    ext : PackagePath.Path -> Try(PackagePath.Path, [IsDirPath, EndsInDots])
    ext = |path| path.ext()

    ## Adds a separator and a string component to the path.
    join : PackagePath.Path, Str -> PackagePath.Path
    join = |path, str| path.join(str)

    ## Expose the raw OS-specific representation.
    to_raw : PackagePath.Path -> [UnixBytes(List(U8)), WindowsU16s(List(U16))]
    to_raw = |path| path.to_raw()

    ## Build a path from the raw OS-specific representation.
    from_raw : [UnixBytes(List(U8)), WindowsU16s(List(U16))] -> PackagePath.Path
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
            Err(err) => Err(err)
        }

    ## Return the type of the path if the path exists on disk.
    ##
    ## > [`File.type`](File#type!) does the same thing, except it takes a `Str` instead of a path value.
    type! : PackagePath.Path => Try([IsFile, IsDir, IsSymLink], [PathErr(IOErr), ..])
    type! = |path| {
        # TODO: once https://github.com/roc-lang/roc/issues/9864 is fixed,
        # app authors should be able to call this effect as `path.type!()`.
        Host.path_type!(to_bytes(path))
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
}

to_bytes : PackagePath.Path -> List(U8)
to_bytes = |path|
    match PackagePath.to_raw(path) {
        UnixBytes(bytes) => bytes
        WindowsU16s(_) => Str.to_utf8(PackagePath.display(path))
    }

import IOErr exposing [IOErr]
import Host
import path.Path as PackagePath

Dir := [].{
    ## Creates a new, empty directory at the provided path.
    ##
    ## If the parent directories do not exist, they will not be created.
    ## Use [Dir.create_all!] to create parent directories as needed.
    create! : PackagePath.Path => Try({}, [DirErr(IOErr), ..])
    create! = |path| widen_dir_err(Host.dir_create!(PackagePath.to_raw(path)))

    ## Creates a new, empty directory at the provided path, including any parent directories.
    ##
    ## If the directory already exists, this will succeed without error.
    create_all! : PackagePath.Path => Try({}, [DirErr(IOErr), ..])
    create_all! = |path| widen_dir_err(Host.dir_create_all!(PackagePath.to_raw(path)))

    ## Deletes an empty directory.
    ##
    ## Fails if the directory is not empty. Use [Dir.delete_all!] to delete
    ## a directory and all its contents.
    delete_empty! : PackagePath.Path => Try({}, [DirErr(IOErr), ..])
    delete_empty! = |path| widen_dir_err(Host.dir_delete_empty!(PackagePath.to_raw(path)))

    ## Deletes a directory and all of its contents recursively.
    ##
    ## Use with caution!
    delete_all! : PackagePath.Path => Try({}, [DirErr(IOErr), ..])
    delete_all! = |path| widen_dir_err(Host.dir_delete_all!(PackagePath.to_raw(path)))

    ## Lists the contents of a directory.
    ##
    ## Returns the byte-preserving paths of all files and directories within
    ## the specified directory.
    list! : PackagePath.Path => Try(List(PackagePath.Path), [DirErr(IOErr), ..])
    list! = |path|
        match Host.dir_list!(PackagePath.to_raw(path)) {
            Ok(raw_list) => Ok(raw_list.map(PackagePath.from_raw)),
            Err(DirErr(err)) => Err(DirErr(err)),
        }
}

widen_dir_err : Try(a, [DirErr(IOErr)]) -> Try(a, [DirErr(IOErr), ..])
widen_dir_err = |result|
    match result {
        Ok(value) => Ok(value),
        Err(DirErr(err)) => Err(DirErr(err)),
    }

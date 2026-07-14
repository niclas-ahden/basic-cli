import IOErr exposing [IOErr]
import Host
import Path

Dir := [].{
    ## Creates a new, empty directory at the provided path.
    ##
    ## If the parent directories do not exist, they will not be created.
    ## Use [Dir.create_all!] to create parent directories as needed.
    create! : Path.Path => Try({}, [DirErr(IOErr), ..])
    create! = |path| widen_dir_err(Host.dir_create!(Path.to_raw(path)))

    ## Creates a new, empty directory at the provided path, including any parent directories.
    ##
    ## If the directory already exists, this will succeed without error.
    create_all! : Path.Path => Try({}, [DirErr(IOErr), ..])
    create_all! = |path| widen_dir_err(Host.dir_create_all!(Path.to_raw(path)))

    ## Deletes an empty directory.
    ##
    ## Fails if the directory is not empty. Use [Dir.delete_all!] to delete
    ## a directory and all its contents.
    delete_empty! : Path.Path => Try({}, [DirErr(IOErr), ..])
    delete_empty! = |path| widen_dir_err(Host.dir_delete_empty!(Path.to_raw(path)))

    ## Deletes a directory and all of its contents recursively.
    ##
    ## Use with caution!
    delete_all! : Path.Path => Try({}, [DirErr(IOErr), ..])
    delete_all! = |path| widen_dir_err(Host.dir_delete_all!(Path.to_raw(path)))

    ## Lists the contents of a directory.
    ##
    ## Returns the byte-preserving paths of all files and directories within
    ## the specified directory.
    list! : Path.Path => Try(List(Path.Path), [DirErr(IOErr), ..])
    list! = |path|
        match Host.dir_list!(Path.to_raw(path)) {
            Ok(raw_list) => Ok(raw_list.map(Path.from_raw)),
            Err(DirErr(err)) => Err(DirErr(err)),
        }
}

widen_dir_err : Try(a, [DirErr(IOErr)]) -> Try(a, [DirErr(IOErr), ..])
widen_dir_err = |result|
    match result {
        Ok(value) => Ok(value),
        Err(DirErr(err)) => Err(DirErr(err)),
    }

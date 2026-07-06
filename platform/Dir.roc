import IOErr exposing [IOErr]
import Host
import path.Path as PackagePath

Dir := [].{
    ## Creates a new, empty directory at the provided path.
    ##
    ## If the parent directories do not exist, they will not be created.
    ## Use [Dir.create_all!] to create parent directories as needed.
    create! : Str => Try({}, [DirErr(IOErr), ..])
    create! = |path| Ok(Host.dir_create!(path)?)

    ## Creates a new, empty directory at the provided path, including any parent directories.
    ##
    ## If the directory already exists, this will succeed without error.
    create_all! : Str => Try({}, [DirErr(IOErr), ..])
    create_all! = |path| Ok(Host.dir_create_all!(path)?)

    ## Deletes an empty directory.
    ##
    ## Fails if the directory is not empty. Use [Dir.delete_all!] to delete
    ## a directory and all its contents.
    delete_empty! : Str => Try({}, [DirErr(IOErr), ..])
    delete_empty! = |path| Ok(Host.dir_delete_empty!(path)?)

    ## Deletes a directory and all of its contents recursively.
    ##
    ## Use with caution!
    delete_all! : Str => Try({}, [DirErr(IOErr), ..])
    delete_all! = |path| Ok(Host.dir_delete_all!(path)?)

    ## Lists the contents of a directory.
    ##
    ## Returns the byte-preserving paths of all files and directories within
    ## the specified directory.
    list! : Str => Try(List(PackagePath.Path), [DirErr(IOErr), ..])
    list! = |path| {
        bytes_list = Host.dir_list!(path)?
        Ok(bytes_list.map(|bytes| PackagePath.unix_bytes(bytes)))
    }
}

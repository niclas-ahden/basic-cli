import Host
import path.Path as PackagePath

Env := [].{
    ## Reads the given environment variable.
    ##
    ## If the value is invalid Unicode, the invalid parts will be replaced with the
    ## [Unicode replacement character](https://unicode.org/glossary/#replacement_character).
    ##
    ## Returns `Err(VarNotFound(name))` if the variable is not set.
    var! : Str => Try(Str, [VarNotFound(Str), ..])
    var! = |name| Ok(Host.env_var!(name)?)

    ## Reads the [current working directory](https://en.wikipedia.org/wiki/Working_directory)
    ## from the environment.
    ##
    ## Returns `Err(CwdUnavailable)` if the cwd cannot be determined.
    cwd! : () => Try(PackagePath.Path, [CwdUnavailable, ..])
    cwd! = || {
        bytes = Host.env_cwd!()?
        Ok(PackagePath.unix_bytes(bytes))
    }

    ## Gets the path to the currently-running executable.
    ##
    ## Returns `Err(ExePathUnavailable)` if the path cannot be determined.
    exe_path! : () => Try(PackagePath.Path, [ExePathUnavailable, ..])
    exe_path! = || {
        bytes = Host.env_exe_path!()?
        Ok(PackagePath.unix_bytes(bytes))
    }

    ## Gets the default directory for temporary files.
    temp_dir! : () => PackagePath.Path
    temp_dir! = || PackagePath.unix_bytes(Host.env_temp_dir!())
}

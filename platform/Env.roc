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
    var! = |name| widen_var_err(Host.env_var!(name))

    ## Reads the [current working directory](https://en.wikipedia.org/wiki/Working_directory)
    ## from the environment.
    ##
    ## Returns `Err(CwdUnavailable)` if the cwd cannot be determined.
    cwd! : () => Try(PackagePath.Path, [CwdUnavailable, ..])
    cwd! = ||
        match Host.env_cwd!() {
            Ok(bytes) => Ok(PackagePath.unix_bytes(bytes)),
            Err(CwdUnavailable) => Err(CwdUnavailable),
        }

    ## Gets the path to the currently-running executable.
    ##
    ## Returns `Err(ExePathUnavailable)` if the path cannot be determined.
    exe_path! : () => Try(PackagePath.Path, [ExePathUnavailable, ..])
    exe_path! = ||
        match Host.env_exe_path!() {
            Ok(bytes) => Ok(PackagePath.unix_bytes(bytes)),
            Err(ExePathUnavailable) => Err(ExePathUnavailable),
        }

    ## Gets the default directory for temporary files.
    temp_dir! : () => PackagePath.Path
    temp_dir! = || PackagePath.unix_bytes(Host.env_temp_dir!())
}

widen_var_err : Try(a, [VarNotFound(Str)]) -> Try(a, [VarNotFound(Str), ..])
widen_var_err = |result|
    match result {
        Ok(value) => Ok(value),
        Err(VarNotFound(name)) => Err(VarNotFound(name)),
    }

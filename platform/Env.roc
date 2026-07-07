import Host
import IOErr exposing [IOErr]
import OsStr exposing [OsStr]
import path.Path as PackagePath

Env := [].{
    ## Reads the given environment variable.
    ##
    ## Returns `Err(VarNotFound(name))` if the variable is not set.
    var! : OsStr => Try(OsStr, [VarNotFound(OsStr), EnvErr(IOErr), ..])
    var! = |name|
        match Host.env_var!(OsStr.to_raw(name)) {
            Ok(raw) => Ok(OsStr.from_raw(raw))
            Err(VarNotFound(raw_name)) => Err(VarNotFound(OsStr.from_raw(raw_name)))
            Err(EnvErr(err)) => Err(EnvErr(err))
        }

    ## Reads the given environment variable as a string if its native value is valid text.
    var_str! : OsStr => Try(Str, [VarNotFound(OsStr), EnvErr(IOErr), InvalidStr(U64), ..])
    var_str! = |name|
        match var!(name) {
            Ok(value) =>
                match OsStr.to_str_try(value) {
                    Ok(str) => Ok(str)
                    Err(InvalidStr(index)) => Err(InvalidStr(index))
                }
            Err(VarNotFound(raw_name)) => Err(VarNotFound(raw_name))
            Err(EnvErr(err)) => Err(EnvErr(err))
        }

    ## Reads the [current working directory](https://en.wikipedia.org/wiki/Working_directory)
    ## from the environment.
    ##
    ## Returns `Err(CwdUnavailable)` if the cwd cannot be determined.
    cwd! : () => Try(PackagePath.Path, [CwdUnavailable, ..])
    cwd! = ||
        match Host.env_cwd!() {
            Ok(raw) => Ok(PackagePath.from_raw(raw)),
            Err(CwdUnavailable) => Err(CwdUnavailable),
        }

    ## Gets the path to the currently-running executable.
    ##
    ## Returns `Err(ExePathUnavailable)` if the path cannot be determined.
    exe_path! : () => Try(PackagePath.Path, [ExePathUnavailable, ..])
    exe_path! = ||
        match Host.env_exe_path!() {
            Ok(raw) => Ok(PackagePath.from_raw(raw)),
            Err(ExePathUnavailable) => Err(ExePathUnavailable),
        }

    ## Gets the default directory for temporary files.
    temp_dir! : () => PackagePath.Path
    temp_dir! = || PackagePath.from_raw(Host.env_temp_dir!())
}

app [main!] { pf: platform "../platform/main.roc" }

import pf.Env
import pf.Path
import pf.Stdout

main! : List(Str) => Try({}, _)
main! = |_args| {
    Stdout.line!("Testing Env module functions...")?

    Stdout.line!("\nTesting Env.var!:")?
    env_var = Env.var!("BASIC_CLI_ENV_TEST")?
    Stdout.line!("BASIC_CLI_ENV_TEST: ${env_var}")?

    Stdout.line!("\nTesting Env.cwd!:")?
    cwd = Env.cwd!()?
    Stdout.line!("cwd: ${Path.display(cwd)}")?

    Stdout.line!("\nTesting Env.exe_path!:")?
    exe_path = Env.exe_path!()?
    Stdout.line!("exe_path: ${Path.display(exe_path)}")?

    Stdout.line!("\nTesting Env.temp_dir!:")?
    temp_dir = Env.temp_dir!()
    Stdout.line!("temp_dir: ${Path.display(temp_dir)}")?

    Stdout.line!("\nAll tests executed.")?

    Ok({})
}

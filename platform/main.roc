platform ""
    requires {} { main! : List(Str) => Try({}, [Exit(I32), ..]) }
    exposes [Cmd, Dir, Env, File, IOErr, Locale, Path, Random, Sleep, Stdin, Stdout, Stderr, Tty, Utc]
    packages {}
    provides { "roc_main": main_for_host! }
    hosted {
        "hosted_cmd_host_exec_exit_code": Cmd.host_exec_exit_code!,
        "hosted_cmd_host_exec_output": Cmd.host_exec_output!,
        "hosted_dir_create": Dir.create!,
        "hosted_dir_create_all": Dir.create_all!,
        "hosted_dir_delete_all": Dir.delete_all!,
        "hosted_dir_delete_empty": Dir.delete_empty!,
        "hosted_dir_list": Dir.list!,
        "hosted_env_cwd": Env.cwd!,
        "hosted_env_exe_path": Env.exe_path!,
        "hosted_env_temp_dir": Env.temp_dir!,
        "hosted_env_var": Env.var!,
        "hosted_file_delete": File.delete!,
        "hosted_file_is_executable": File.is_executable!,
        "hosted_file_is_readable": File.is_readable!,
        "hosted_file_is_writable": File.is_writable!,
        "hosted_file_read_bytes": File.read_bytes!,
        "hosted_file_read_utf8": File.read_utf8!,
        "hosted_file_size_in_bytes": File.size_in_bytes!,
        "hosted_file_time_accessed": File.time_accessed!,
        "hosted_file_time_created": File.time_created!,
        "hosted_file_time_modified": File.time_modified!,
        "hosted_file_write_bytes": File.write_bytes!,
        "hosted_file_write_utf8": File.write_utf8!,
        "hosted_locale_all": Locale.all!,
        "hosted_locale_get": Locale.get!,
        "hosted_path_type": Path.host_path_type!,
        "hosted_random_seed_u32": Random.seed_u32!,
        "hosted_random_seed_u64": Random.seed_u64!,
        "hosted_sleep_millis": Sleep.millis!,
        "hosted_stderr_line": Stderr.line!,
        "hosted_stderr_write": Stderr.write!,
        "hosted_stderr_write_bytes": Stderr.write_bytes!,
        "hosted_stdin_bytes": Stdin.bytes!,
        "hosted_stdin_line": Stdin.line!,
        "hosted_stdin_read_to_end": Stdin.read_to_end!,
        "hosted_stdout_line": Stdout.line!,
        "hosted_stdout_write": Stdout.write!,
        "hosted_stdout_write_bytes": Stdout.write_bytes!,
        "hosted_tty_disable_raw_mode": Tty.disable_raw_mode!,
        "hosted_tty_enable_raw_mode": Tty.enable_raw_mode!,
        "hosted_utc_now": Utc.now!,
    }
    targets: {
        inputs_dir: "targets/",
        x64mac: { inputs: ["libhost.a", app] },
        arm64mac: { inputs: ["libhost.a", app] },
        x64musl: { inputs: ["crt1.o", "libhost.a", "libunwind.a", app, "libc.a"] },
        arm64musl: { inputs: ["crt1.o", "libhost.a", "libunwind.a", app, "libc.a"] },
    }

import Cmd
import Dir
import Env
import File
import IOErr
import Locale
import Path
import Random
import Sleep
import Stdin
import Stdout
import Stderr
import Tty
import Utc

main_for_host! : List(Str) => I32
main_for_host! = |args|
    match main!(args) {
        Ok({}) => 0
        Err(Exit(code)) => code
        Err(other) =>
            match Stderr.line!("Program exited with error: ${Str.inspect(other)}") {
                _ => 1
            }
    }

platform ""
    requires {} { main! : List(Str) => Try({}, [Exit(I32), ..]) }
    exposes [Cmd, Dir, Env, File, Http, IOErr, InternalSqlite, Locale, Path, Random, Sleep, Sqlite, Stdin, Stdout, Stderr, Tcp, Tty, Utc]
    packages {
        # HTTP data types (Method, Request, Response) come from the shared
        # roc-lang/http package so apps and other packages using it see the same
        # nominal types. The platform supplies only the effectful `Http.send!`.
        http: "https://github.com/roc-lang/http/releases/download/0.1/6LcdNq2r7xTBwj972ecYWUkMWobJr94yL2NyJpHRAXap.tar.zst",
        # Path data types and pure helpers come from the shared roc-lang/path
        # package so path values have the same nominal type across packages.
        path: "http://127.0.0.1:38095/8p8iryUUorAFTUDeqYcwc9bFYSwpbVqhYpuHvRAS5Cq4.tar.zst",
    }
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
        "hosted_file_open_reader": File.host_open_reader!,
        "hosted_file_read_line": File.host_read_line!,
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
        # SQLite hosted functions are kept at the end so adding them does not
        # renumber the generated glue types for the modules declared above.
        "hosted_sqlite_prepare": Sqlite.host_prepare!,
        "hosted_sqlite_bind": Sqlite.host_bind!,
        "hosted_sqlite_columns": Sqlite.host_columns!,
        "hosted_sqlite_column_value": Sqlite.host_column_value!,
        "hosted_sqlite_step": Sqlite.host_step!,
        "hosted_sqlite_reset": Sqlite.host_reset!,
        # TCP hosted functions are likewise kept at the end to avoid renumbering.
        "hosted_tcp_connect": Tcp.host_connect!,
        "hosted_tcp_read_up_to": Tcp.host_read_up_to!,
        "hosted_tcp_read_exactly": Tcp.host_read_exactly!,
        "hosted_tcp_read_until": Tcp.host_read_until!,
        "hosted_tcp_write": Tcp.host_write!,
        # HTTP is likewise kept at the end to avoid renumbering glue types.
        "hosted_http_send_request": Http.host_send_request!,
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
import Http
import IOErr
import InternalSqlite
import Locale
import Path
import Random
import Sleep
import Sqlite
import Stdin
import Stdout
import Stderr
import Tcp
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

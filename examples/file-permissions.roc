app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# To run this example: check the README.md in this folder

main! = |_args| {
    file = "LICENSE"

    is_executable = File.is_executable!(file)?

    is_readable = File.is_readable!(file)?

    is_writable = File.is_writable!(file)?

    Stdout.line!(
        \\${file} file permissions:
        \\    Executable: ${Str.inspect(is_executable)}
        \\    Readable: ${Str.inspect(is_readable)}
        \\    Writable: ${Str.inspect(is_writable)}
    )
}

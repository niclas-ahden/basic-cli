app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File
import pf.Utc

# To run this example: check the README.md in this folder

main! = |_args| {
    file = "LICENSE"

    # NOTE: these functions will not work if basic-cli was built with musl, which is the case for the normal tar.br URL release.
    # See https://github.com/roc-lang/basic-cli?tab=readme-ov-file#running-locally to build basic-cli without musl.

    time_modified = Utc.to_millis_since_epoch(File.time_modified!(file)?)
    time_accessed = Utc.to_millis_since_epoch(File.time_accessed!(file)?)
    time_created = Utc.to_millis_since_epoch(File.time_created!(file)?)

    Stdout.line!(
        \\${file} file time metadata:
        \\    Modified: ${time_modified.to_str()} ms since epoch
        \\    Accessed: ${time_accessed.to_str()} ms since epoch
        \\    Created: ${time_created.to_str()} ms since epoch
    )
}

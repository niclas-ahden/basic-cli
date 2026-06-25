app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# Demo of File.read_utf8! and File.write_utf8!

main! = |_args| {
    out_file = "out.txt"

    _ = Stdout.line!("Writing a string to out.txt")

    file_result = || {
        File.write_utf8!(out_file, "a string!")?

        contents = File.read_utf8!(out_file)?

        _ = Stdout.line!("I read the file back. Its contents are: \"${contents}\"")

        # Cleanup
        File.delete!(out_file)?

        Ok({})
    }

    match file_result() {
        Ok({}) => Ok({})
        Err(_) => {
            _ = Stdout.line!("Error during file operations")
            Err(Exit(1))
        }
    }
}

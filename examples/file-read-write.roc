app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# Demo of File.read_utf8! and File.write_utf8!

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    out_file = "out.txt"

    Stdout.line!("Writing a string to out.txt") ? |_| Exit(1)

    file_result = || {
        File.write_utf8!(out_file, "a string!")?

        contents = File.read_utf8!(out_file)?

        # Cleanup
        File.delete!(out_file)?

        Ok(contents)
    }

    match file_result() {
        Ok(contents) => {
            Stdout.line!("I read the file back. Its contents are: \"${contents}\"") ? |_| Exit(1)
            Ok({})
        }
        Err(_) => {
            Stdout.line!("Error during file operations") ? |_| Exit(1)
            Err(Exit(1))
        }
    }
}

app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# Demo of File.read_utf8! and File.write_utf8!

main! : List(Str) => Try({}, [Exit(I32), FileErr(File.IOErr), ..])
main! = |_args| {
    out_file = "out.txt"

    Stdout.line!("Writing a string to out.txt")

    match File.write_utf8!(out_file, "a string!") {
        Ok({}) => {}
        Err(FileErr(err)) => return Err(FileErr(err))
    }

    contents = match File.read_utf8!(out_file) {
        Ok(value) => value
        Err(FileErr(err)) => return Err(FileErr(err))
    }

    Stdout.line!("I read the file back. Its contents are: \"${contents}\"")

    # Cleanup
    match File.delete!(out_file) {
        Ok({}) => {}
        Err(FileErr(err)) => return Err(FileErr(err))
    }

    Ok({})
}

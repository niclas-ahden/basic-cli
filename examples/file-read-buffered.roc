app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# To run this example: check the README.md in this folder

# # Buffered File Reading
#
# Instead of reading an entire file and storing all of it in memory,
# like with File.read_utf8, you may want to read it in parts.
# A part of the file is stored in a buffer.
# Typically you process a part and then you ask for the next one.
#
# This can be useful to process large files without using a lot of RAM or
# requiring the user to wait until the complete file is processed when they
# only wanted to look at the first page.
#
# See examples/file-read-write.roc if you want to read the full contents at once.

main! = |_args| {
    read_file = || {
        reader = File.open_reader!("LICENSE")?

        read_summary = process_line!(reader, { lines_read: 0, bytes_read: 0 })?

        _ = Stdout.line!("Done reading file: ${Str.inspect(read_summary)}")
        Ok({})
    }

    match read_file() {
        Ok({}) => Ok({})
        Err(err) => {
            _ = Stdout.line!("Error during buffered file read: ${Str.inspect(err)}")
            Err(Exit(1))
        }
    }
}

ReadSummary : {
    lines_read : U64,
    bytes_read : U64,
}

## Count the number of lines and the number of bytes read.
process_line! : File.Reader, ReadSummary => Try(ReadSummary, _)
process_line! = |reader, { lines_read, bytes_read }|
    match File.read_line!(reader) {
        Ok(bytes) if List.len(bytes) == 0 =>
            Ok({ lines_read, bytes_read })

        Ok(bytes) =>
            process_line!(
                reader,
                {
                    lines_read: lines_read + 1,
                    bytes_read: bytes_read + List.len(bytes),
                },
            )

        Err(err) => Err(err)
    }

app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr
import pf.Stdout
import pf.File
import pf.Path

# # Buffered File Reading
#
# Instead of reading an entire file and storing all of it in memory,
# like with `Path.read_utf8!`, you may want to read it in parts.
# A part of the file is stored in a buffer.
# Typically you process a part and then you ask for the next one.
#
# This can be useful to process large files without using a lot of RAM or
# requiring the user to wait until the complete file is processed when they
# only wanted to look at the first page.
#
# See `examples/file-read-write.roc` if you want to read the full contents at once.

main! : List(OsStr) => Try({}, _)
main! = |_args| {

	reader : File.Reader
	reader = File.open_reader!("LICENSE")?

	read_summary : ReadSummary
	read_summary = process_line!(reader, { lines_read: 0, bytes_read: 0 })?

	Stdout.line!("Done reading file: ${Str.inspect(read_summary)}")?

	Ok({})
}

ReadSummary := { lines_read : U64, bytes_read : U64 }

## Count the number of lines and the number of bytes read.
process_line! : File.Reader, ReadSummary => Try(ReadSummary, _)
process_line! = |reader, { lines_read, bytes_read }|
	match reader.read_line!() {
		Ok(bytes) if bytes.len() == 0 =>
			Ok({ lines_read, bytes_read })

		Ok(bytes) =>
			process_line!(
				reader,
				{
					lines_read: lines_read + 1,
					bytes_read: bytes_read + bytes.len(),
				},
			)

		Err(err) => Err(err)
	}

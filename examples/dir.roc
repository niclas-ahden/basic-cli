app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Dir
import pf.Path

# Demo of all Dir functions.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# Best-effort cleanup from a previous interrupted run.
	Dir.delete_all!("empty-dir") ?? {}
	Dir.delete_all!("nested-dir") ?? {}

	# Create a directory
	Dir.create!("empty-dir")?

	# Create a directory and its parents
	Dir.create_all!("nested-dir/a/b/c")?

	# Create a child directory
	Dir.create!("nested-dir/child")?

	# List the contents of a directory
	paths = Dir.list!("nested-dir")?

	# Check the contents of the directory
	expect paths.len() == 2
	expect List.contains(paths.map(Path.display), "nested-dir/a")
	expect List.contains(paths.map(Path.display), "nested-dir/child")

	# Delete an empty directory
	Dir.delete_empty!("empty-dir")?

	# Delete all directories recursively
	Dir.delete_all!("nested-dir")?

	Stdout.line!("Success!")?
	Ok({})
}

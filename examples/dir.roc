app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Dir
import pf.Path

# Create and inspect a small project workspace.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	# Best-effort cleanup from a previous interrupted run.
	Dir.delete_all!("demo-workspace") ?? {}

	# Create a directory
	Dir.create!("demo-workspace")?

	# Create a directory and its parents
	Dir.create_all!("demo-workspace/src/components")?

	# Create a child directory
	Dir.create!("demo-workspace/assets")?

	# List the contents of a directory
	paths = Dir.list!("demo-workspace")?
	displayed = paths.map(Path.display)

	# Check the contents of the directory
	expect paths.len() == 2
	expect List.contains(displayed, "demo-workspace/src")
	expect List.contains(displayed, "demo-workspace/assets")

	Stdout.line!("Workspace entries: ${Str.join_with(displayed, ", ")}")?

	# Delete all directories recursively
	Dir.delete_all!("demo-workspace")?

	Stdout.line!("Workspace cleaned up.")?
	Ok({})
}

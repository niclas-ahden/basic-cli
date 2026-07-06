app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Stderr
import pf.Dir

# Demo of all Dir functions.

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    dir_result = || {
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
        expect List.len(paths) == 2
        expect List.contains(paths, "nested-dir/a")
        expect List.contains(paths, "nested-dir/child")

        # Delete an empty directory
        Dir.delete_empty!("empty-dir")?

        # Delete all directories recursively
        Dir.delete_all!("nested-dir")?

        Ok({})
    }

    match dir_result() {
        Ok(_) => {
            Stdout.line!("Success!") ? |_| Exit(1)
            Ok({})
        }
        Err(err) => {
            Stderr.line!("Error during directory operations: ${Str.inspect(err)}") ? |_| Exit(1)
            Err(Exit(1))
        }
    }
}

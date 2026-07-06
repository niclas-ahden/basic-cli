app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.File

# Demonstrates error handling patterns

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    file_name = "test-file.txt"

    # Try to read a file that doesn't exist - should error
    result = File.read_utf8!("nonexistent-file.txt")
    match result {
        Ok(content) => {
            Stdout.line!("Unexpected success: ${content}") ? |_| Exit(1)
        }
        Err(FileErr(NotFound)) => {
            Stdout.line!("Expected error: File not found (NotFound)") ? |_| Exit(1)
        }
        Err(FileErr(PermissionDenied)) => {
            Stdout.line!("Error: Permission denied") ? |_| Exit(1)
        }
        Err(FileErr(Other(msg))) => {
            Stdout.line!("Error: ${msg}") ? |_| Exit(1)
        }
        Err(_) => {
            Stdout.line!("Error: Other file error") ? |_| Exit(1)
        }
    }

    # Now demonstrate success path - create, read, then cleanup
    file_result = || {
        File.write_utf8!(file_name, "Hello from error-handling example!")?

        content = File.read_utf8!(file_name)?

        # Cleanup
        File.delete!(file_name)?

        Ok(content)
    }

    match file_result() {
        Ok(content) => {
            Stdout.line!("${file_name} contains: ${content}") ? |_| Exit(1)
            Ok({})
        }
        Err(_) => {
            Stdout.line!("Error during file operations") ? |_| Exit(1)
            Err(Exit(1))
        }
    }
}

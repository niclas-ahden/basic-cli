app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.File
import pf.Path

# Demonstrates error handling patterns

main! : List(OsStr) => Try({}, _)
main! = |_args| {
    file_name = "test-file.txt"
    file_display = Path.display(file_name)

    # Try to read a file that doesn't exist - should error
    result = File.read_utf8!("nonexistent-file.txt")
    match result {
        Ok(content) => Err(UnexpectedReadSuccess(content))?
        Err(FileErr(NotFound)) => {
            Stdout.line!("Expected error: File not found (NotFound)")?
        }
        Err(FileErr(PermissionDenied)) => {
            Stdout.line!("Error: Permission denied")?
        }
        Err(FileErr(Other(msg))) => {
            Stdout.line!("Error: ${msg}")?
        }
        Err(_) => {
            Stdout.line!("Error: Other file error")?
        }
    }

    File.write_utf8!(file_name, "Hello from error-handling example!") ? |err| FileWriteFailed(err)

    content = File.read_utf8!(file_name) ? |err| FileReadFailed(err)

    # Cleanup
    File.delete!(file_name) ? |err| FileDeleteFailed(err)

    Stdout.line!("${file_display} contains: ${content}")?
    Ok({})
}

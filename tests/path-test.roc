app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Cmd
import pf.Path
import pf.Stdout

main! : List(OsStr) => Try({}, [Exit(I32), ..])
main! = |_args| {
    result = run_tests!()
    cleanup_result = cleanup_test_files!()

    match result {
        Ok({}) => {
            cleanup_result ? |_| Exit(1)
            Ok({})
        }
        Err(err) => {
            _ = cleanup_result
            Stdout.line!("Test run failed: ${Str.inspect(err)}") ? |_| Exit(1)
            Err(Exit(1))
        }
    }
}

run_tests! : () => Try({}, _)
run_tests! = || {
    Stdout.line!("Testing Path functions...")?
    Stdout.line!("This will create and manipulate test files and directories in the current directory.")?
    Stdout.line!("")?

    test_path_creation!()?
    test_file_operations!()?
    test_directory_operations!()?
    test_hard_link!()?
    test_path_rename!()?
    test_path_exists!()?

    Stdout.line!("\nI ran all Path function tests.")?
    Ok({})
}

test_path_creation! : () => Try({}, _)
test_path_creation! = || {
    Stdout.line!("Testing Path.from_bytes and Path.with_extension:")?

    path_bytes = [116, 101, 115, 116, 95, 112, 97, 116, 104]
    path_from_bytes = Path.from_bytes(path_bytes)
    expected_str = "test_path"
    actual_str = Path.display(path_from_bytes)

    base_path = "test_file"
    path_with_ext = Path.with_extension(base_path, "txt")

    path_with_dot = "test_file."
    path_dot_ext = Path.with_extension(path_with_dot, "json")

    path_replace_ext = "test_file.old"
    path_new_ext = Path.with_extension(path_replace_ext, "new")

    Stdout.line!("Created path from bytes: ${Path.display(path_from_bytes)}")?
    Stdout.line!("Path.from_bytes result matches expected: ${bool_to_old_str(actual_str == expected_str)}")?
    Stdout.line!("Path with extension: ${Path.display(path_with_ext)}")?
    Stdout.line!("Extension added correctly: ${bool_to_old_str(Path.display(path_with_ext) == "test_file.txt")}")?
    Stdout.line!("Path with dot and extension: ${Path.display(path_dot_ext)}")?
    Stdout.line!("Extension after dot: ${bool_to_old_str(Path.display(path_dot_ext) == "test_file.json")}")?
    Stdout.line!("Path with replaced extension: ${Path.display(path_new_ext)}")?
    Stdout.line!("Extension replaced: ${bool_to_old_str(Path.display(path_new_ext) == "test_file.new")}")?

    Ok({})
}

test_file_operations! : () => Try({}, _)
test_file_operations! = || {
    Stdout.line!("\nTesting Path file operations:")?

    test_bytes = [72, 101, 108, 108, 111, 44, 32, 80, 97, 116, 104, 33]
    bytes_path = "test_path_bytes.txt"
    Path.write_bytes!(test_bytes, bytes_path)?

    read_bytes = Path.read_bytes!(bytes_path)?

    Stdout.line!("Bytes written: ${Str.inspect(test_bytes)}")?
    Stdout.line!("Bytes read: ${Str.inspect(read_bytes)}")?
    Stdout.line!("Bytes match: ${bool_to_old_str(test_bytes == read_bytes)}")?

    utf8_content = "Hello from Path module! 🚀"
    utf8_path = "test_path_utf8.txt"
    Path.write_utf8!(utf8_content, utf8_path)?

    cat_output = Cmd.new("cat").arg("test_path_utf8.txt").exec_output!()?
    read_utf8 = Path.read_utf8!(utf8_path)?

    Stdout.line!("File content via cat: ${cat_output.stdout_utf8}")?
    Stdout.line!("UTF-8 written: ${utf8_content}")?
    Stdout.line!("UTF-8 read: ${read_utf8}")?
    Stdout.line!("UTF-8 content matches: ${bool_to_old_str(utf8_content == read_utf8)}")?

    json_content = "{\"message\":\"Path test\",\"numbers\":[1,2,3]}"
    json_path = "test_path_json.json"
    Path.write_utf8!(json_content, json_path)?
    read_json = Path.read_utf8!(json_path)?

    Stdout.line!("JSON content: ${read_json}")?
    Stdout.line!("JSON contains 'message' field: ${bool_to_old_str(Str.contains(read_json, "\"message\""))}")?
    Stdout.line!("JSON contains 'numbers' field: ${bool_to_old_str(Str.contains(read_json, "\"numbers\""))}")?

    delete_path = "test_to_delete.txt"
    Path.write_utf8!("This file will be deleted", delete_path)?
    Path.delete!(delete_path)?

    exists_after_delete = Path.exists!(delete_path)?
    Stdout.line!("File no longer exists: ${bool_to_old_str(Bool.not(exists_after_delete))}")?

    Ok({})
}

test_directory_operations! : () => Try({}, _)
test_directory_operations! = || {
    Stdout.line!("\nTesting Path directory operations...")?

    single_dir = "test_single_dir"
    Path.create_dir!(single_dir)?

    nested_dir = "test_parent/test_child/test_grandchild"
    Path.create_all!(nested_dir)?

    find_output = Cmd.new("find").args(["test_parent", "-type", "d"]).exec_output!()?
    dir_count = List.len(Str.split_on(find_output.stdout_utf8, "\n")) - 1

    Stdout.line!("Nested directory structure:")?
    Stdout.write!(find_output.stdout_utf8)?
    Stdout.line!("")?
    Stdout.line!("Number of directories created: ${U64.to_str(dir_count)}")?

    Path.write_utf8!("File 1", "test_single_dir/file1.txt")?
    Path.write_utf8!("File 2", "test_single_dir/file2.txt")?
    Path.create_dir!("test_single_dir/subdir")?

    ls_contents = Cmd.new("ls").args(["-la", "test_single_dir"]).exec_output!()?
    Stdout.line!("Directory contents:")?
    Stdout.write!(ls_contents.stdout_utf8)?
    Stdout.line!("")?

    empty_dir = "test_empty_dir"
    Path.create_dir!(empty_dir)?
    Path.delete_empty!(empty_dir)?
    empty_exists_after_delete = Path.exists!(empty_dir)?
    Stdout.line!("Empty dir was deleted: ${bool_to_old_str(Bool.not(empty_exists_after_delete))}")?

    du_output = Cmd.new("du").args(["-sh", "test_parent"]).exec_output!()?
    Path.delete_all!("test_parent")?
    parent_exists_after_delete = Path.exists!("test_parent")?

    Stdout.write!("Size before delete_all: ${du_output.stdout_utf8}")?
    Stdout.line!("")?
    Stdout.line!("Parent dir no longer exists: ${bool_to_old_str(Bool.not(parent_exists_after_delete))}")?

    Path.delete_all!(single_dir)?
    Ok({})
}

get_hard_link_count! : Str => Try(Str, _)
get_hard_link_count! = |path_str| {
    ls_l = Cmd.new("ls").args_str(["-l", path_str]).exec_output!()?
    parts = Str.split_on(ls_l.stdout_utf8, " ")
    non_empty_parts = List.keep_if(parts, |part| !Str.is_empty(part))
    count = non_empty_parts.get(1) ? |_| HardLinkCountNotFound
    Ok(count)
}

test_hard_link! : () => Try({}, _)
test_hard_link! = || {
    Stdout.line!("\nTesting Path.hard_link!:")?

    original_path = "test_path_original.txt"
    Path.write_utf8!("Original content for Path hard link test", original_path)?

    hard_link_count_before = get_hard_link_count!("test_path_original.txt")?

    link_path = "test_path_hardlink.txt"
    Path.hard_link!(original_path, link_path)?

    hard_link_count_after = get_hard_link_count!("test_path_original.txt")?
    original_content = Path.read_utf8!(original_path)?
    link_content = Path.read_utf8!(link_path)?

    Stdout.line!("Hard link count before: ${hard_link_count_before}")?
    Stdout.line!("Hard link count after: ${hard_link_count_after}")?
    Stdout.line!("Original content: ${original_content}")?
    Stdout.line!("Link content: ${link_content}")?
    Stdout.line!("Content matches: ${bool_to_old_str(original_content == link_content)}")?

    ls_li_output =
        Cmd.new("ls")
        .args(["-li", "test_path_original.txt", "test_path_hardlink.txt"])
        .exec_output!()?

    lines = Str.split_on(ls_li_output.stdout_utf8, "\n")
    non_empty_lines = List.keep_if(lines, |line| !Str.is_empty(line))
    inodes =
        List.map(non_empty_lines, |line| {
            parts = Str.split_on(line, " ")
            non_empty_parts = List.keep_if(parts, |part| !Str.is_empty(part))
            non_empty_parts.take_first(1)
        })

    first_inode = inodes.get(0) ? |_| FirstInodeNotFound
    second_inode = inodes.get(1) ? |_| SecondInodeNotFound

    Stdout.line!("Inode information:")?
    Stdout.write!(ls_li_output.stdout_utf8)?
    Stdout.line!("")?
    Stdout.line!("First file inode: ${Str.inspect(first_inode)}")?
    Stdout.line!("Second file inode: ${Str.inspect(second_inode)}")?
    Stdout.line!("Inodes are equal: ${bool_to_old_str(first_inode == second_inode)}")?

    Ok({})
}

test_path_rename! : () => Try({}, _)
test_path_rename! = || {
    Stdout.line!("\nTesting Path.rename!:")?

    original_path = "test_path_rename_original.txt"
    new_path = "test_path_rename_new.txt"
    test_file_content = "Content for rename test."

    Path.write_utf8!(test_file_content, original_path)?
    Path.rename!(original_path, new_path)?

    original_file_exists_after = Path.is_file!(original_path)?
    if original_file_exists_after {
        return Err(TestFailed("Original file still exists after rename"))
    } else {
        Stdout.line!("✓ Original file no longer exists")?
    }

    new_file_exists = Path.is_file!(new_path)?
    if new_file_exists {
        Stdout.line!("✓ Renamed file exists")?
    } else {
        return Err(TestFailed("Renamed file does not exist"))
    }

    content = Path.read_utf8!(new_path)?
    if content == test_file_content {
        Stdout.line!("✓ Renamed file has correct content")?
    } else {
        return Err(TestFailed("Renamed file has incorrect content"))
    }

    Ok({})
}

test_path_exists! : () => Try({}, _)
test_path_exists! = || {
    Stdout.line!("\nTesting Path.exists!:")?

    filename = "test_path_exists.txt"
    Path.write_utf8!("This file exists", filename)?

    file_exists = Path.exists!(filename)?
    if file_exists {
        Stdout.line!("✓ Path.exists! returns true for a file that exists")?
    } else {
        return Err(TestFailed("Path.exists! returned false for a file that exists"))
    }

    # Keep receiver dispatch for platform effects covered by an active test.
    path_type = filename.type!()?
    match path_type {
        IsFile => {}
        _ => return Err(TestFailed("Path.type! did not identify the file"))
    }

    Path.delete!(filename)?

    file_exists_after_delete = Path.exists!(filename)?
    if file_exists_after_delete {
        return Err(TestFailed("Path.exists! returned true for a file that does not exist"))
    } else {
        Stdout.line!("✓ Path.exists! returns false for a file that does not exist")?
    }

    Ok({})
}

cleanup_test_files! : () => Try({}, _)
cleanup_test_files! = || {
    Stdout.line!("\nCleaning up test files...")?

    test_files = [
        "test_path_bytes.txt",
        "test_path_hardlink.txt",
        "test_path_json.json",
        "test_path_original.txt",
        "test_path_rename_new.txt",
        "test_path_utf8.txt",
    ]

    ls_before_cleanup =
        Cmd.new("ls")
        .args([
            "-la",
            "test_path_bytes.txt",
            "test_path_hardlink.txt",
            "test_path_json.json",
            "test_path_original.txt",
            "test_path_rename_new.txt",
            "test_path_utf8.txt",
        ])
        .exec_output!()

    match ls_before_cleanup {
        Ok(output) => {
            Stdout.line!("Files to clean up:")?
            Stdout.write!(output.stdout_utf8)?
            Stdout.line!("")?
        }
        Err(_) => {}
    }

    ls_after_cleanup_command = "ls test_path_bytes.txt test_path_hardlink.txt test_path_json.json test_path_original.txt test_path_rename_new.txt test_path_utf8.txt || true"
    files_deleted = delete_paths_and_confirm!(test_files, 0)?

    ls_after_cleanup =
        Cmd.new("sh")
        .args_str(["-c", ls_after_cleanup_command])
        .exec_output!()?

    Stdout.write!(ls_after_cleanup.stderr_utf8_lossy)?

    Stdout.line!("Files deleted successfully: ${bool_to_old_str(files_deleted)}")?
    Ok({})
}

delete_paths_and_confirm! : List(Str), U64 => Try(Bool, _)
delete_paths_and_confirm! = |filenames, index| {
    if index >= List.len(filenames) {
        Ok(True)
    } else {
        filename = filenames.get(index) ? |_| CleanupFileMissing
        _ = Path.delete!(Path.from_str(filename))
        exists = Path.exists!(Path.from_str(filename))?
        rest_deleted = delete_paths_and_confirm!(filenames, index + 1)?

        Ok(if exists { False } else { rest_deleted })
    }
}

bool_to_old_str : Bool -> Str
bool_to_old_str = |value|
    if value {
        "Bool.true"
    } else {
        "Bool.false"
    }

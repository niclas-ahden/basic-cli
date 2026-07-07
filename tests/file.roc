app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Cmd
import pf.File
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
    Stdout.line!("Testing some File functions...")?
    Stdout.line!("This will create and manipulate test files in the current directory.")?
    Stdout.line!("")?

    test_basic_file_operations!()?
    test_file_type_checking!()?
    test_file_reader_with_capacity!()?
    test_hard_link!()?
    test_file_rename!()?
    test_file_exists!()?

    Stdout.line!("\nI ran all file function tests.")?

    Ok({})
}

test_basic_file_operations! : () => Try({}, _)
test_basic_file_operations! = || {
    Stdout.line!("Testing File.write_bytes! and File.read_bytes!:")?

    test_bytes = [72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33]
    bytes_path = "test_bytes.txt"
    File.write_bytes!(bytes_path, test_bytes)?

    file_content_bytes = File.read_bytes!(bytes_path)?
    Stdout.line!("Bytes in test_bytes.txt: ${Str.inspect(file_content_bytes)}")?

    Stdout.line!("\nTesting File.write!:")?

    write_path = "test_write.json"
    File.write_utf8!(write_path, "{\"some\":\"json stuff\"}")?
    json_file_content = File.read_utf8!(write_path)?
    Stdout.line!("Content of test_write.json: ${json_file_content}")?

    Ok({})
}

test_file_type_checking! : () => Try({}, _)
test_file_type_checking! = || {
    Stdout.line!("\nTesting File.is_file!:")?
    bytes_path = "test_bytes.txt"
    symlink_path = "test_symlink.txt"
    is_file_result = File.is_file!(bytes_path)?
    if is_file_result {
        Stdout.line!("✓ test_bytes.txt is confirmed to be a file")?
    } else {
        return Err(TestFailed("test_bytes.txt is not recognized as a file"))
    }

    Stdout.line!("\nTesting File.is_sym_link!:")?
    is_symlink_one = File.is_sym_link!(bytes_path)?
    if is_symlink_one {
        return Err(TestFailed("test_bytes.txt is a symbolic link"))
    } else {
        Stdout.line!("✓ test_bytes.txt is not a symbolic link")?
    }

    Cmd.exec!("ln", ["-s", "test_bytes.txt", "test_symlink.txt"])?

    is_symlink_two = File.is_sym_link!(symlink_path)?
    if is_symlink_two {
        Stdout.line!("✓ test_symlink.txt is a symbolic link")?
    } else {
        return Err(TestFailed("test_symlink.txt is not a symbolic link"))
    }

    Stdout.line!("\nTesting File.type!:")?

    file_type_file = File.type!(bytes_path)?
    Stdout.line!("test_bytes.txt file type: ${Str.inspect(file_type_file)}")?

    file_type_dir = File.type!(".")?
    Stdout.line!(". file type: ${Str.inspect(file_type_dir)}")?

    file_type_symlink = File.type!(symlink_path)?
    Stdout.line!("test_symlink.txt file type: ${Str.inspect(file_type_symlink)}")?

    Ok({})
}

test_file_reader_with_capacity! : () => Try({}, _)
test_file_reader_with_capacity! = || {
    Stdout.line!("\nTesting File.open_reader_with_capacity!:")?

    multi_line_content = "First line\nSecond line\nThird line\n"
    multiline_path = "test_multiline.txt"
    File.write_utf8!(multiline_path, multi_line_content)?

    reader_buf_size = 3
    reader = File.open_reader_with_capacity!(multiline_path, reader_buf_size)?
    Stdout.line!("✓ Successfully opened reader with ${U64.to_str(reader_buf_size)} byte capacity")?

    Stdout.line!("\nReading lines from file:")?
    line1_bytes = File.read_line!(reader)?
    line1_str = Str.from_utf8(line1_bytes)?
    Stdout.line!("Line 1: ${line1_str}")?

    line2_bytes = File.read_line!(reader)?
    line2_str = Str.from_utf8(line2_bytes)?
    Stdout.line!("Line 2: ${line2_str}")?

    Ok({})
}

test_hard_link! : () => Try({}, _)
test_hard_link! = || {
    Stdout.line!("\nTesting File.hard_link!:")?

    original_path = "test_original_file.txt"
    link_path = "test_link_to_original.txt"
    File.write_utf8!(original_path, "Original file content for hard link test")?
    File.hard_link!(original_path, link_path)?
    Stdout.line!("✓ Successfully created hard link: test_link_to_original.txt")?

    ls_li_output =
        Cmd.new("ls")
        .args(["-li", "test_original_file.txt", "test_link_to_original.txt"])
        .exec_output!()?

    lines = Str.split_on(ls_li_output.stdout_utf8, "\n")
    non_empty_lines = List.keep_if(lines, |line| !Str.is_empty(line))
    inodes =
        List.map(non_empty_lines, |line| {
            parts = Str.split_on(line, " ")
            non_empty_parts = List.keep_if(parts, |part| !Str.is_empty(part))
            List.take_first(non_empty_parts, 1)
        })

    first_inode = inodes.get(0) ? |_| FirstInodeNotFound
    second_inode = inodes.get(1) ? |_| SecondInodeNotFound

    Stdout.line!("Hard link inodes should be equal: ${bool_to_old_str(first_inode == second_inode)}")?

    original_content = File.read_utf8!(original_path)?
    link_content = File.read_utf8!(link_path)?

    if original_content == link_content {
        Stdout.line!("✓ Hard link contains same content as original")?
    } else {
        return Err(TestFailed("hard link content differs from original"))
    }

    Ok({})
}

test_file_rename! : () => Try({}, _)
test_file_rename! = || {
    Stdout.line!("\nTesting File.rename!:")?

    original_name = "test_rename_original.txt"
    new_name = "test_rename_new.txt"
    original_display = Path.display(original_name)
    new_display = Path.display(new_name)
    File.write_utf8!(original_name, "Content for rename test")?

    File.rename!(original_name, new_name)?
    Stdout.line!("✓ Successfully renamed ${original_display} to ${new_display}")?

    original_exists_after = File.is_file!(original_name)?
    if original_exists_after {
        return Err(TestFailed("original file still exists after rename"))
    } else {
        Stdout.line!("✓ Original file ${original_display} no longer exists")?
    }

    new_exists = File.is_file!(new_name)?
    if new_exists {
        Stdout.line!("✓ Renamed file ${new_display} exists")?
    } else {
        return Err(TestFailed("renamed file does not exist"))
    }

    content = File.read_utf8!(new_name)?
    if content == "Content for rename test" {
        Stdout.line!("✓ Renamed file has correct content")?
    } else {
        return Err(TestFailed("renamed file has incorrect content"))
    }

    Ok({})
}

test_file_exists! : () => Try({}, _)
test_file_exists! = || {
    Stdout.line!("\nTesting File.exists!:")?

    filename = "test_exists.txt"
    File.write_utf8!(filename, "")?

    test_file_exists = File.exists!(filename)?
    if test_file_exists {
        Stdout.line!("✓ File.exists! returns true for a file that exists")?
    } else {
        return Err(TestFailed("File.exists! returned false for a file that exists"))
    }

    File.delete!(filename)?

    test_file_exists_after_delete = File.exists!(filename)?
    if test_file_exists_after_delete {
        return Err(TestFailed("File.exists! returned true for a file that does not exist"))
    } else {
        Stdout.line!("✓ File.exists! returns false for a file that does not exist")?
    }

    Ok({})
}

cleanup_test_files! : () => Try({}, _)
cleanup_test_files! = || {
    Stdout.line!("\nCleaning up test files...")?

    test_files = [
        "test_bytes.txt",
        "test_symlink.txt",
        "test_write.json",
        "test_multiline.txt",
        "test_original_file.txt",
        "test_link_to_original.txt",
        "test_rename_new.txt",
    ]

    for filename in test_files {
        _ = File.delete!(Path.from_str(filename))
    }

    Stdout.line!("✓ Deleted all files.")?
    Ok({})
}

bool_to_old_str : Bool -> Str
bool_to_old_str = |value|
    if value {
        "Bool.true"
    } else {
        "Bool.false"
    }

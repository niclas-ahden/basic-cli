app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Env
import pf.Stdout
import pf.Sqlite
import pf.Path

# To run this example: check the README.md in this folder and set `export DB_PATH=./examples/todos2.db`

# Demo of the wider Sqlite API: queries, decoders, nullable columns, inserts,
# updates, deletes, prepared statements.

# Sql that was used to create the table:
# CREATE TABLE todos (
#     id INTEGER PRIMARY KEY AUTOINCREMENT,
#     task TEXT NOT NULL,
#     status TEXT NOT NULL,
#     edited BOOLEAN,
# );
# Note 1: the edited column is nullable, this is for demonstration purposes only.
# We recommend using `NOT NULL` when possible.
# Note 2: boolean is "fake" in sqlite https://www.sqlite.org/datatype3.html

main! : List(OsStr) => Try({}, _)
main! = |_args| run!()

run! : () => Try({}, _)
run! = || {
	db_path = 
		match Env.var!("DB_PATH") {
			Ok(p) => Path.from_os_str(p)
			Err(_) => "./examples/todos2.db"
		}

	# Example: print all rows
	all_todos = Sqlite.query_many!(
		{
			path: db_path,
			query: "SELECT * FROM todos;",
			bindings: [],
			rows: decode_full_todo,
		},
	) ? |err| QueryAllTodosFailed(err)

	print_line!("All Todos:")?
	for t in all_todos {
		print_line!("    id: ${I64.to_str(t.id)}, task: ${t.task}, status: ${status_to_str(t.status)}, edited: ${edited_to_str(decode_edited(t.edited_val))}")?
	}

	# Example: filter rows by status (decode a single column)
	tasks_in_progress = Sqlite.query_many!(
		{
			path: db_path,
			query: "SELECT id, task, status FROM todos WHERE status = :status;",
			bindings: [{ name: ":status", value: encode_status(InProgress) }],
			rows: Sqlite.str("task"),
		},
	) ? |err| QueryInProgressTasksFailed(err)

	print_line!("")?
	print_line!("In-progress Todos:")?
	for task in tasks_in_progress {
		print_line!("    In-progress task: ${task}")?
	}

	# Example: insert a row
	Sqlite.execute!(
		{
			path: db_path,
			query: "INSERT INTO todos (task, status, edited) VALUES (:task, :status, :edited);",
			bindings: [
				{ name: ":task", value: String("Make sql example.") },
				{ name: ":status", value: encode_status(InProgress) },
				{ name: ":edited", value: encode_edited(NotEdited) },
			],
		},
	) ? |err| InsertTodoFailed(err)

	# Example: insert multiple rows from a Roc list
	todos_list = [
		{ task: "Insert Roc list 1", status: Todo, edited: NotEdited },
		{ task: "Insert Roc list 2", status: Todo, edited: NotEdited },
		{ task: "Insert Roc list 3", status: Todo, edited: NotEdited },
	]

	values_str = 
		Str.join_with(
			List.map_with_index(
				todos_list,
				|_t, indx| {
					i = U64.to_str(indx)
					"(:task${i}, :status${i}, :edited${i})"
				},
			),
			", ",
		)

	binding_groups = 
		List.map_with_index(
			todos_list,
			|t, indx| {
				i = U64.to_str(indx)
				[
					{ name: ":task${i}", value: String(t.task) },
					{ name: ":status${i}", value: encode_status(t.status) },
					{ name: ":edited${i}", value: encode_edited(t.edited) },
				]
			},
		)

	all_bindings = 
		Iter.fold(List.iter(binding_groups), [], |acc, group| List.concat(acc, group))

	Sqlite.execute!(
		{
			path: db_path,
			query: "INSERT INTO todos (task, status, edited) VALUES ${values_str};",
			bindings: all_bindings,
		},
	) ? |err| InsertTodoListFailed(err)

	# Example: update a row
	Sqlite.execute!(
		{
			path: db_path,
			query: "UPDATE todos SET status = :status WHERE task = :task;",
			bindings: [
				{ name: ":task", value: String("Make sql example.") },
				{ name: ":status", value: encode_status(Completed) },
			],
		},
	) ? |err| UpdateTodoFailed(err)

	# Example: delete a row
	Sqlite.execute!(
		{
			path: db_path,
			query: "DELETE FROM todos WHERE task = :task;",
			bindings: [{ name: ":task", value: String("Make sql example.") }],
		},
	) ? |err| DeleteTodoFailed(err)

	# Example: delete all rows where ID is greater than 3 (cleanup so this example is repeatable)
	Sqlite.execute!(
		{
			path: db_path,
			query: "DELETE FROM todos WHERE id > :id;",
			bindings: [{ name: ":id", value: Integer(3) }],
		},
	) ? |err| CleanupInsertedTodosFailed(err)

	# Example: count the number of rows
	count = Sqlite.query!(
		{
			path: db_path,
			query: "SELECT COUNT(*) as \"count\" FROM todos;",
			bindings: [],
			row: Sqlite.u64("count"),
		},
	) ? |err| CountTodosFailed(err)

	print_line!("")?
	print_line!("Row count: ${U64.to_str(count)}")?

	# Example: prepared statements
	# Note: This is faster if you execute the same prepared statement many times.
	prepared_query = Sqlite.prepare!(
		{
			path: db_path,
			# sort by the length of the task description
			query: "SELECT * FROM todos ORDER BY LENGTH(task);",
		},
	) ? |err| PrepareSortedTodosFailed(err)

	todos_sorted = Sqlite.query_many_prepared!(
		{
			stmt: prepared_query,
			bindings: [],
			rows: decode_task_status,
		},
	) ? |err| QuerySortedTodosFailed(err)

	print_line!("")?
	print_line!("Todos sorted by length of task description:")?
	for t in todos_sorted {
		print_line!("    task: ${t.task}, status: ${status_to_str(t.status)}")?
	}

	Ok({})
}

print_line! : Str => Try({}, _)
print_line! = |s| Stdout.line!(s)

# Decode every column of the todos table. The nullable `edited` column is returned
# raw (`[NotNull(I64), Null]`) and interpreted by `decode_edited` at the call site:
# decoding both `status` (via `?`) and `edited` inside this nested decoder lambda
# currently panics the type checker, so we keep only one interpreting `?` here.
decode_full_todo = |cols|
	|stmt| {
		id = Sqlite.i64("id")(cols)(stmt)?
		task = Sqlite.str("task")(cols)(stmt)?
		status_str = Sqlite.str("status")(cols)(stmt)?
		match decode_status(status_str) {
			Ok(status) => {
				edited_val = Sqlite.nullable_i64("edited")(cols)(stmt)?
				Ok({ id, task, status, edited_val })
			}
			Err(ParseError(message)) => Err(ParseError(message))
		}
	}

# Decode just the task and status columns.
decode_task_status = |cols|
	|stmt| {
		task = Sqlite.str("task")(cols)(stmt)?
		status_str = Sqlite.str("status")(cols)(stmt)?
		match decode_status(status_str) {
			Ok(status) => Ok({ task, status })
			Err(ParseError(message)) => Err(ParseError(message))
		}
	}

TodoStatus : [Todo, Completed, InProgress]

decode_status = |status_str|
	match status_str {
		"todo" => Ok(Todo)
		"completed" => Ok(Completed)
		"in-progress" => Ok(InProgress)
		_ => Err(ParseError("Unknown status str: ${status_str}"))
	}

status_to_str : TodoStatus -> Str
status_to_str = |status|
	match status {
		Todo => "todo"
		Completed => "completed"
		InProgress => "in-progress"
	}

encode_status = |status| String(status_to_str(status))

EditedValue : [Edited, NotEdited, Unknown]

decode_edited = |edited_val|
	match edited_val {
		NotNull(1) => Edited
		NotNull(0) => NotEdited
		_ => Unknown
	}

edited_to_str : EditedValue -> Str
edited_to_str = |edited|
	match edited {
		Edited => "edited"
		NotEdited => "not-edited"
		Unknown => "unknown"
	}

encode_edited = |edited|
	match edited {
		Edited => Integer(1)
		NotEdited => Integer(0)
		Unknown => Null
	}

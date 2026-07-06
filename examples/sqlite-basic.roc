app [main!] { pf: platform "../platform/main.roc" }

import pf.Env
import pf.Stdout
import pf.Sqlite

# To run this example: check the README.md in this folder and set `export DB_PATH=./examples/todos.db`

# Demo of basic Sqlite usage

# Sql to create the table:
# CREATE TABLE todos (
#     id INTEGER PRIMARY KEY AUTOINCREMENT,
#     task TEXT NOT NULL,
#     status TEXT NOT NULL
# );

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
    match run!() {
        Ok(_) => Ok({})
        Err(_) => Err(Exit(1))
    }

run! : () => Try({}, [SqliteExampleFailed(Str)])
run! = || {
    db_path =
        match Env.var!("DB_PATH") {
            Ok(p) => p
            Err(_) => "./examples/todos.db"
        }

    todos = query_todos_by_status!(db_path, "todo") ? |err| SqliteExampleFailed(Str.inspect(err))

    print_line!("All Todos:")?

    for todo in todos {
        print_todo!(todo)?
    }

    completed_todos = query_todos_by_status!(db_path, "completed") ? |err| SqliteExampleFailed(Str.inspect(err))

    print_line!("")?
    print_line!("Completed Todos:")?
    for todo in completed_todos {
        print_todo!(todo)?
    }

    Ok({})
}

Todo : { id : Str, status : TodoStatus, task : Str }

print_todo! : Todo => Try({}, [SqliteExampleFailed(Str)])
print_todo! = |todo|
    print_line!("    id: ${todo.id}, task: ${todo.task}, status: ${status_to_str(todo.status)}")

print_line! : Str => Try({}, [SqliteExampleFailed(Str)])
print_line! = |line| {
    Stdout.line!(line) ? |_| SqliteExampleFailed("stdout write failed")
    Ok({})
}

query_todos_by_status! = |db_path, status|
    Sqlite.query_many!(
        {
            path: db_path,
            query: "SELECT id, task, status FROM todos WHERE status = :status;",
            bindings: [{ name: ":status", value: String(status) }],
            rows: decode_todo,
        },
    )

# A row decoder is `List(Str) -> (Stmt => Try(a, err))`; the new compiler does not
# support the record-builder (`<-`) sugar, so we combine the leaf decoders by hand.
decode_todo = |cols|
    |stmt| {
        id = Sqlite.i64("id")(cols)(stmt)?
        task = Sqlite.str("task")(cols)(stmt)?
        status_str = Sqlite.str("status")(cols)(stmt)?
        match decode_todo_status(status_str) {
            Ok(status) => Ok({ id: I64.to_str(id), task, status })
            Err(ParseError(message)) => Err(ParseError(message))
        }
    }

TodoStatus : [Todo, Completed, InProgress]

status_to_str : TodoStatus -> Str
status_to_str = |status|
    match status {
        Todo => "Todo"
        Completed => "Completed"
        InProgress => "InProgress"
    }

decode_todo_status = |status_str|
    match status_str {
        "todo" => Ok(Todo)
        "completed" => Ok(Completed)
        "in-progress" => Ok(InProgress)
        _ => Err(ParseError("Unknown status str: ${status_str}"))
    }

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

main! = |_args| {
    db_path =
        match Env.var!("DB_PATH") {
            Ok(p) => p
            Err(_) => "./examples/todos.db"
        }

    todos = query_todos_by_status!(db_path, "todo")?

    _h1 = Stdout.line!("All Todos:")

    List.for_each!(todos, |todo| print_todo!(todo))

    completed_todos = query_todos_by_status!(db_path, "completed")?

    _h2 = Stdout.line!("")
    _h3 = Stdout.line!("Completed Todos:")
    List.for_each!(completed_todos, |todo| print_todo!(todo))

    Ok({})
}

Todo : { id : Str, status : TodoStatus, task : Str }

print_todo! = |todo|
    match Stdout.line!("    id: ${todo.id}, task: ${todo.task}, status: ${status_to_str(todo.status)}") {
        _ => {}
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
        status = decode_todo_status(status_str)?
        Ok({ id: I64.to_str(id), task, status })
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

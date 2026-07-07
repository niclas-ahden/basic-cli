app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Env
import pf.Sqlite
import pf.Path
import pf.Stdout

main! : List(OsStr) => Try({}, [Exit(I32), ..])
main! = |_args|
    match run!() {
        Ok({}) => Ok({})
        Err(err) => {
            Stdout.line!("Test run failed: ${Str.inspect(err)}") ? |_| Exit(1)
            Err(Exit(1))
        }
    }

run! : () => Try({}, _)
run! = || {
    db_path = Path.from_os_str(Env.var!("DB_PATH")?)

    setup_db!(db_path)?

    all_rows = Sqlite.query_many!(
        {
            path: db_path,
            query: "SELECT * FROM test;",
            bindings: [],
            rows: decode_test_row,
        },
    )?

    rows_texts = List.map(all_rows, format_test_row)
    rows_texts_str = Str.join_with(rows_texts, "\n")

    Stdout.line!("Rows: ${rows_texts_str}")?

    prepared_count = Sqlite.prepare!(
        {
            path: db_path,
            query: "SELECT COUNT(*) as \"count\" FROM test;",
        },
    )?

    count = Sqlite.query_prepared!(
        {
            stmt: prepared_count,
            bindings: [],
            row: Sqlite.u64("count"),
        },
    )?

    Stdout.line!("Row count: ${U64.to_str(count)}")?

    prepared_update = Sqlite.prepare!(
        {
            path: db_path,
            query: "UPDATE test SET col_text = :col_text WHERE id = :id;",
        },
    )?

    Sqlite.execute_prepared!(
        {
            stmt: prepared_update,
            bindings: [
                { name: ":id", value: Integer(1) },
                { name: ":col_text", value: String("Updated text 1") },
            ],
        },
    )?

    Sqlite.execute_prepared!(
        {
            stmt: prepared_update,
            bindings: [
                { name: ":id", value: Integer(2) },
                { name: ":col_text", value: String("Updated text 2") },
            ],
        },
    )?

    updated_rows = Sqlite.query_many!(
        {
            path: db_path,
            query: "SELECT col_text FROM test;",
            bindings: [],
            rows: Sqlite.str("col_text"),
        },
    )?

    Stdout.line!("Updated rows: ${Str.inspect(updated_rows)}")?

    Sqlite.execute_prepared!(
        {
            stmt: prepared_update,
            bindings: [
                { name: ":id", value: Integer(1) },
                { name: ":col_text", value: String("example text") },
            ],
        },
    )?

    Sqlite.execute_prepared!(
        {
            stmt: prepared_update,
            bindings: [
                { name: ":id", value: Integer(2) },
                { name: ":col_text", value: String("sample text") },
            ],
        },
    )?

    tagged_value_test = Sqlite.query_many!(
        {
            path: db_path,
            query: "SELECT * FROM test;",
            bindings: [],
            rows: Sqlite.tagged_value("col_text"),
        },
    )?

    tagged_value_texts = List.map(tagged_value_test, format_tagged_value)
    Stdout.line!("Tagged value test: [${Str.join_with(tagged_value_texts, ", ")}]")?

    sql_res = Sqlite.execute!(
        {
            path: db_path,
            query: "UPDATE test SET id = :id WHERE col_text = :col_text;",
            bindings: [
                { name: ":col_text", value: String("sample text") },
                { name: ":id", value: String("This should be an integer") },
            ],
        },
    )

    mismatch_result =
        match sql_res {
            Ok(_) => Err(TestFailed("Expected a Sqlite type mismatch error"))
            Err(SqliteErr(err_type, _)) => {
                Stdout.line!("Error: ${Sqlite.errcode_to_str(err_type)}")?
                Ok({})
            }
            Err(err) => Err(TestFailed("Expected a Sqlite error, got ${Str.inspect(err)}"))
        }
    mismatch_result?

    Stdout.line!("Success!")?
    Ok({})
}

setup_db! = |db_path| {
    Sqlite.execute!(
        {
            path: db_path,
            query: "DROP TABLE IF EXISTS test;",
            bindings: [],
        },
    )?

    Sqlite.execute!(
        {
            path: db_path,
            query: "CREATE TABLE test (id INTEGER PRIMARY KEY AUTOINCREMENT, col_text TEXT NOT NULL, col_bytes BLOB NOT NULL, col_i32 INTEGER NOT NULL, col_i16 INTEGER NOT NULL, col_i8 INTEGER NOT NULL, col_u32 INTEGER NOT NULL, col_u16 INTEGER NOT NULL, col_u8 INTEGER NOT NULL, col_f64 REAL NOT NULL, col_f32 REAL NOT NULL, col_nullable_str TEXT, col_nullable_bytes BLOB, col_nullable_i64 INTEGER, col_nullable_i32 INTEGER, col_nullable_i16 INTEGER, col_nullable_i8 INTEGER, col_nullable_u64 INTEGER, col_nullable_u32 INTEGER, col_nullable_u16 INTEGER, col_nullable_u8 INTEGER, col_nullable_f64 REAL, col_nullable_f32 REAL);",
            bindings: [],
        },
    )?

    prepared_insert = Sqlite.prepare!(
        {
            path: db_path,
            query: "INSERT INTO test (id, col_text, col_bytes, col_i32, col_i16, col_i8, col_u32, col_u16, col_u8, col_f64, col_f32, col_nullable_str, col_nullable_bytes, col_nullable_i64, col_nullable_i32, col_nullable_i16, col_nullable_i8, col_nullable_u64, col_nullable_u32, col_nullable_u16, col_nullable_u8, col_nullable_f64, col_nullable_f32) VALUES (:id, :col_text, :col_bytes, :col_i32, :col_i16, :col_i8, :col_u32, :col_u16, :col_u8, :col_f64, :col_f32, :col_nullable_str, :col_nullable_bytes, :col_nullable_i64, :col_nullable_i32, :col_nullable_i16, :col_nullable_i8, :col_nullable_u64, :col_nullable_u32, :col_nullable_u16, :col_nullable_u8, :col_nullable_f64, :col_nullable_f32);",
        },
    )?

    Sqlite.execute_prepared!(
        {
            stmt: prepared_insert,
            bindings: [
                { name: ":id", value: Integer(1) },
                { name: ":col_text", value: String("example text") },
                { name: ":col_bytes", value: Bytes([72, 101, 108, 108, 111]) },
                { name: ":col_i32", value: Integer(123456) },
                { name: ":col_i16", value: Integer(1234) },
                { name: ":col_i8", value: Integer(123) },
                { name: ":col_u32", value: Integer(654321) },
                { name: ":col_u16", value: Integer(4321) },
                { name: ":col_u8", value: Integer(234) },
                { name: ":col_f64", value: Real(123.456) },
                { name: ":col_f32", value: Real(78.9) },
                { name: ":col_nullable_str", value: String("nullable text") },
                { name: ":col_nullable_bytes", value: Bytes([119, 111, 114, 108, 100]) },
                { name: ":col_nullable_i64", value: Integer(987654321) },
                { name: ":col_nullable_i32", value: Integer(456789) },
                { name: ":col_nullable_i16", value: Integer(5678) },
                { name: ":col_nullable_i8", value: Integer(56) },
                { name: ":col_nullable_u64", value: Integer(123456789) },
                { name: ":col_nullable_u32", value: Integer(987654) },
                { name: ":col_nullable_u16", value: Integer(8765) },
                { name: ":col_nullable_u8", value: Integer(78) },
                { name: ":col_nullable_f64", value: Real(456.789) },
                { name: ":col_nullable_f32", value: Real(12.34) },
            ],
        },
    )?

    Sqlite.execute_prepared!(
        {
            stmt: prepared_insert,
            bindings: [
                { name: ":id", value: Integer(2) },
                { name: ":col_text", value: String("sample text") },
                { name: ":col_bytes", value: Bytes([119, 111, 114, 108, 100]) },
                { name: ":col_i32", value: Integer(789012) },
                { name: ":col_i16", value: Integer(5678) },
                { name: ":col_i8", value: Integer(45) },
                { name: ":col_u32", value: Integer(1234567) },
                { name: ":col_u16", value: Integer(9876) },
                { name: ":col_u8", value: Integer(123) },
                { name: ":col_f64", value: Real(456.789) },
                { name: ":col_f32", value: Real(23.45) },
                { name: ":col_nullable_str", value: Null },
                { name: ":col_nullable_bytes", value: Null },
                { name: ":col_nullable_i64", value: Null },
                { name: ":col_nullable_i32", value: Integer(123456) },
                { name: ":col_nullable_i16", value: Null },
                { name: ":col_nullable_i8", value: Null },
                { name: ":col_nullable_u64", value: Null },
                { name: ":col_nullable_u32", value: Integer(654321) },
                { name: ":col_nullable_u16", value: Null },
                { name: ":col_nullable_u8", value: Null },
                { name: ":col_nullable_f64", value: Null },
                { name: ":col_nullable_f32", value: Real(67.89) },
            ],
        },
    )?

    Ok({})
}

decode_test_row = |cols|
    |stmt| {
        col_text = Sqlite.str("col_text")(cols)(stmt)?
        col_bytes = Sqlite.bytes("col_bytes")(cols)(stmt)?
        col_i32 = Sqlite.i32("col_i32")(cols)(stmt)?
        col_i16 = Sqlite.i16("col_i16")(cols)(stmt)?
        col_i8 = Sqlite.i8("col_i8")(cols)(stmt)?
        col_u32 = Sqlite.u32("col_u32")(cols)(stmt)?
        col_u16 = Sqlite.u16("col_u16")(cols)(stmt)?
        col_u8 = Sqlite.u8("col_u8")(cols)(stmt)?
        col_f64 = Sqlite.f64("col_f64")(cols)(stmt)?
        col_f32 = Sqlite.f64("col_f32")(cols)(stmt)?
        col_nullable_str = Sqlite.nullable_str("col_nullable_str")(cols)(stmt)?
        col_nullable_bytes = Sqlite.nullable_bytes("col_nullable_bytes")(cols)(stmt)?
        col_nullable_i64 = Sqlite.nullable_i64("col_nullable_i64")(cols)(stmt)?
        col_nullable_i32 = Sqlite.nullable_i32("col_nullable_i32")(cols)(stmt)?
        col_nullable_i16 = Sqlite.nullable_i16("col_nullable_i16")(cols)(stmt)?
        col_nullable_i8 = Sqlite.nullable_i8("col_nullable_i8")(cols)(stmt)?
        col_nullable_u64 = Sqlite.nullable_u64("col_nullable_u64")(cols)(stmt)?
        col_nullable_u32 = Sqlite.nullable_u32("col_nullable_u32")(cols)(stmt)?
        col_nullable_u16 = Sqlite.nullable_u16("col_nullable_u16")(cols)(stmt)?
        col_nullable_u8 = Sqlite.nullable_u8("col_nullable_u8")(cols)(stmt)?
        col_nullable_f64 = Sqlite.nullable_f64("col_nullable_f64")(cols)(stmt)?
        col_nullable_f32 = Sqlite.nullable_f64("col_nullable_f32")(cols)(stmt)?

        Ok({
            col_text,
            col_bytes,
            col_i32,
            col_i16,
            col_i8,
            col_u32,
            col_u16,
            col_u8,
            col_f64,
            col_f32,
            col_nullable_str,
            col_nullable_bytes,
            col_nullable_i64,
            col_nullable_i32,
            col_nullable_i16,
            col_nullable_i8,
            col_nullable_u64,
            col_nullable_u32,
            col_nullable_u16,
            col_nullable_u8,
            col_nullable_f64,
        col_nullable_f32,
        })
    }

format_test_row = |row|
    "{col_bytes: ${Str.inspect(row.col_bytes)}, col_f32: ${Str.inspect(row.col_f32)}, col_f64: ${Str.inspect(row.col_f64)}, col_i16: ${Str.inspect(row.col_i16)}, col_i32: ${Str.inspect(row.col_i32)}, col_i8: ${Str.inspect(row.col_i8)}, col_nullable_bytes: ${format_nullable(row.col_nullable_bytes)}, col_nullable_f32: ${format_nullable(row.col_nullable_f32)}, col_nullable_f64: ${format_nullable(row.col_nullable_f64)}, col_nullable_i16: ${format_nullable(row.col_nullable_i16)}, col_nullable_i32: ${format_nullable(row.col_nullable_i32)}, col_nullable_i64: ${format_nullable(row.col_nullable_i64)}, col_nullable_i8: ${format_nullable(row.col_nullable_i8)}, col_nullable_str: ${format_nullable(row.col_nullable_str)}, col_nullable_u16: ${format_nullable(row.col_nullable_u16)}, col_nullable_u32: ${format_nullable(row.col_nullable_u32)}, col_nullable_u64: ${format_nullable(row.col_nullable_u64)}, col_nullable_u8: ${format_nullable(row.col_nullable_u8)}, col_text: \"${row.col_text}\", col_u16: ${Str.inspect(row.col_u16)}, col_u32: ${Str.inspect(row.col_u32)}, col_u8: ${Str.inspect(row.col_u8)}}"

format_nullable = |value|
    match value {
        NotNull(inner) => "(NotNull ${Str.inspect(inner)})"
        Null => "Null"
    }

format_tagged_value = |value|
    match value {
        Bytes(bytes) => "(Bytes ${Str.inspect(bytes)})"
        Integer(int) => "(Integer ${Str.inspect(int)})"
        Null => "Null"
        Real(real) => "(Real ${Str.inspect(real)})"
        String(str) => "(String \"${str}\")"
    }

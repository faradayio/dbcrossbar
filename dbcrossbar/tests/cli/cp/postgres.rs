//! Postgres-specific tests.

use cli_test_dir::*;
use difference::assert_diff;
use std::{fs, path::Path, process::Command};

use super::*;

#[test]
#[ignore]
fn cp_from_postgres_to_exact_csv() {
    let pg_table = post_test_table_url("cp_from_postgres_to_exact_csv");
    assert_cp_to_exact_csv("cp_from_postgres_to_exact_csv", &pg_table);
}

#[test]
#[ignore]
fn cp_csv_to_postgres_append() {
    let testdir = TestDir::new("dbcrossbar", "cp_csv_to_postgres_append");
    let src = testdir.src_path("fixtures/many_types.csv");
    let schema = testdir.src_path("fixtures/many_types.sql");
    let pg_table = post_test_table_url("cp_csv_to_postgres_append");

    // CSV to Postgres.
    testdir
        .cmd()
        .args(&[
            "cp",
            "--if-exists=overwrite",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            &pg_table,
        ])
        .tee_output()
        .expect_success();

    // CSV to Postgres, again, but appending.
    testdir
        .cmd()
        .args(&[
            "cp",
            "--if-exists=append",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            &pg_table,
        ])
        .tee_output()
        .expect_success();
}

#[test]
#[ignore]
fn cp_from_postgres_with_where() {
    let testdir = TestDir::new("dbcrossbar", "cp_from_postgres_with_where");
    let src = testdir.src_path("fixtures/posts.csv");
    let filtered = testdir.src_path("fixtures/posts_where_author_id_1.csv");
    let schema = testdir.src_path("fixtures/posts.sql");
    let pg_table = post_test_table_url("cp_from_postgres_with_where");

    // CSV to Postgres.
    testdir
        .cmd()
        .args(&[
            "cp",
            "--if-exists=overwrite",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            &pg_table,
        ])
        .tee_output()
        .expect_success();

    // PostgreSQL back to CSV using --where.
    testdir
        .cmd()
        .args(&[
            "cp",
            &format!("--schema=postgres-sql:{}", schema.display()),
            "--where",
            "author_id = 1",
            &pg_table,
            "csv:out/",
        ])
        .tee_output()
        .expect_success();

    let expected = fs::read_to_string(&filtered).unwrap();
    let actual =
        fs::read_to_string(testdir.path("out/cp_from_postgres_with_where.csv"))
            .unwrap();
    assert_diff!(&expected, &actual, ",", 0);
}

#[test]
#[ignore]
fn postgres_upsert() {
    let _ = env_logger::try_init();
    let testdir = TestDir::new("dbcrossbar", "postgres_upsert");
    let srcs = &[
        testdir.src_path("fixtures/upsert/upsert_1.csv"),
        testdir.src_path("fixtures/upsert/upsert_2.csv"),
    ];
    let expected = testdir.src_path("fixtures/upsert/upsert_result.csv");
    let schema = testdir.src_path("fixtures/upsert/upsert.sql");
    let pg_table = post_test_table_url("postgres_upsert");

    // CSVes to Postgres.
    let mut first = true;
    for src in srcs {
        let if_exists = if first {
            first = false;
            "--if-exists=overwrite"
        } else {
            // Make sure we have a unique index on key1,key2 first.
            Command::new("psql")
                .arg(postgres_test_url())
                .args(&[
                    "--command",
                    "CREATE UNIQUE INDEX ON postgres_upsert (key1, key2)",
                ])
                .expect_success();

            // Our `--if-exists` argument.
            "--if-exists=upsert-on:key1,key2"
        };
        testdir
            .cmd()
            .args(&[
                "cp",
                if_exists,
                &format!("--schema=postgres-sql:{}", schema.display()),
                &format!("csv:{}", src.display()),
                &pg_table,
            ])
            .tee_output()
            .expect_success();
    }

    // Postgres to CSV.
    testdir
        .cmd()
        .args(&["cp", "--if-exists=overwrite", &pg_table, "csv:out.csv"])
        .tee_output()
        .expect_success();

    // We sort the lines of the CSVs because BigQuery outputs in any order.
    // This has the side effect of putting the headers at the end.
    let normalize_csv = |path: &Path| -> String {
        let text = fs::read_to_string(&path).unwrap();
        let mut lines = text.lines().collect::<Vec<_>>();
        lines.sort_unstable();
        lines.join("\n")
    };
    let expected = normalize_csv(&expected);
    let actual = normalize_csv(&testdir.path("out.csv"));
    assert_diff!(&expected, &actual, ",", 0);
}

#[test]
#[ignore]
fn cp_pg_append_upsert_legacy_json() {
    let testdir = TestDir::new("dbcrossbar", "cp_from_postgres_with_where");
    let src = testdir.src_path("fixtures/legacy_json.csv");
    let schema = testdir.src_path("fixtures/legacy_json.sql");
    let pg_table = post_test_table_url("legacy_json");

    // Create a database table manually, forcing the use of `json` instead of
    // `jsonb`. `dbcrossbar` silently upgrades `json` to `jsonb` under normal
    // circumstances, because it always translates column types into portable
    // types like `DataType::Json`.
    Command::new("psql")
        .arg(postgres_test_url())
        .args(&["--command", "DROP TABLE IF EXISTS legacy_json;"])
        .expect_success();
    Command::new("psql")
        .arg(postgres_test_url())
        .args(&[
            "--command",
            include_str!("../../../fixtures/legacy_json.sql"),
        ])
        .expect_success();

    // CSV to PostgreSQL (append).
    testdir
        .cmd()
        .args(&[
            "cp",
            "--if-exists=append",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            &pg_table,
        ])
        .tee_output()
        .expect_success();

    // CSV to PostgreSQL (upsert).
    testdir
        .cmd()
        .args(&[
            "cp",
            // Also test the short form of `--if-exists=upsert-on:id`.
            "-U",
            "id",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            &pg_table,
        ])
        .tee_output()
        .expect_success();

    // PostgreSQL to CSV.
    testdir
        .cmd()
        .args(&[
            "cp",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &pg_table,
            "csv:out.csv",
        ])
        .tee_output()
        .expect_success();

    let expected = fs::read_to_string(&src).unwrap();
    let actual = fs::read_to_string(testdir.path("out.csv")).unwrap();
    assert_diff!(&expected, &actual, ",", 0);
}

#[test]
#[ignore]
fn cp_pg_tricky_column_types() {
    let testdir = TestDir::new("dbcrossbar", "cp_pg_tricky_column_types");
    let src = testdir.src_path("fixtures/more_pg_types.csv");
    let schema = testdir.src_path("fixtures/more_pg_types.sql");
    let pg_table = post_test_table_url("more_pg_types");

    // Create a database table manually, forcing the use of the actual Postgres
    // types we want to test, and not the nearest `dbcrossbar` portable
    // equivalents.
    Command::new("psql")
        .arg(postgres_test_url())
        .args(&["--command", "DROP TABLE IF EXISTS more_pg_types;"])
        .expect_success();
    Command::new("psql")
        .arg(postgres_test_url())
        .args(&[
            "--command",
            include_str!("../../../fixtures/more_pg_types.sql"),
        ])
        .expect_success();

    // CSV to PostgreSQL.
    testdir
        .cmd()
        .args(&[
            "cp",
            "--if-exists=append",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            &pg_table,
        ])
        .tee_output()
        .expect_success();

    // PostgreSQL to CSV.
    testdir
        .cmd()
        .args(&[
            "cp",
            &format!("--schema=postgres-sql:{}", schema.display()),
            &pg_table,
            "csv:out.csv",
        ])
        .tee_output()
        .expect_success();

    let expected = fs::read_to_string(&src).unwrap();
    let actual = fs::read_to_string(testdir.path("out.csv")).unwrap();
    assert_diff!(&expected, &actual, ",", 0);
}

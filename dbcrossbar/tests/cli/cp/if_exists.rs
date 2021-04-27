//! Tests for `--if-exists` and related arguments.

use cli_test_dir::*;
use std::fs;

use super::*;

#[test]
fn cp_if_exists_mutual_exclusion() {
    let testdir = TestDir::new("dbcrossbar", "cp_if_exists_mutual_exclusion");
    let src = testdir.src_path("fixtures/example.csv");
    testdir
        .cmd()
        .arg("cp")
        .arg("-F")
        .arg("--if-exists=append")
        .arg(&format!("csv:{}", src.display()))
        .arg("csv:out.csv")
        .expect_failure();
}

#[test]
fn cp_if_exists_error() {
    let testdir = TestDir::new("dbcrossbar", "cp_if_exists_error");
    let src = testdir.src_path("fixtures/example.csv");
    let expected = fs::read_to_string(&src).unwrap();
    testdir
        .cmd()
        .arg("cp")
        .arg(&format!("csv:{}", src.display()))
        .arg("csv:out.csv")
        .expect_success();

    for &args in &[&["--if-exists=error"], &[][..]] {
        testdir
            .cmd()
            .arg("cp")
            .args(args)
            .arg(&format!("csv:{}", src.display()))
            .arg("csv:out.csv")
            .expect_failure();
    }

    testdir.expect_file_contents("out.csv", &expected);
}

#[test]
fn cp_if_exists_overwrite() {
    let testdir = TestDir::new("dbcrossbar", "cp_if_exists_overwrite");
    let src = testdir.src_path("fixtures/example.csv");
    let expected = fs::read_to_string(&src).unwrap();

    testdir
        .cmd()
        .arg("cp")
        .arg(&format!("csv:{}", src.display()))
        .arg("csv:out.csv")
        .expect_success();

    for &arg in &["--if-exists=overwrite", "-F"] {
        testdir
            .cmd()
            .arg("cp")
            .arg(arg)
            .arg(&format!("csv:{}", src.display()))
            .arg("csv:out.csv")
            .expect_success();
    }

    testdir.expect_file_contents("out.csv", &expected);
}

#[test]
#[ignore]
fn cp_if_exists_append() {
    let testdir = TestDir::new("dbcrossbar", "cp_if_exists_append");
    let src = testdir.src_path("fixtures/example.csv");
    let pg_table = post_test_table_url("cp_if_exists_append");

    testdir
        .cmd()
        .arg("cp")
        .arg(&format!("csv:{}", src.display()))
        .arg(&pg_table)
        .expect_success();

    for &arg in &["--if-exists=append", "-A"] {
        testdir
            .cmd()
            .arg("cp")
            .arg(arg)
            .arg(&format!("csv:{}", src.display()))
            .arg(&pg_table)
            .expect_success();
    }

    testdir
        .cmd()
        .arg("cp")
        .arg(&pg_table)
        .arg("csv:out.csv")
        .expect_success();
    let expected = r#"id,first_name,last_name
1,John,Doe
1,John,Doe
1,John,Doe
"#;
    testdir.expect_file_contents("out.csv", expected);
}

// upsert-on is tested elsewhere.

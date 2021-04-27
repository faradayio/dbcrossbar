//! Tests for the `cp` subcommand.

use cli_test_dir::*;
use difference::assert_diff;
use std::{env, fs};

mod bigml;
mod bigquery;
mod combined;
mod csv;
mod gs;
mod if_exists;
mod postgres;
mod redshift;
mod s3;
mod shopify;

/// The URL of our test database.
pub(crate) fn postgres_test_url() -> String {
    env::var("POSTGRES_TEST_URL").unwrap_or_else(|_| {
        "postgres://postgres:@localhost:5432/dbcrossbar_test".to_owned()
    })
}

/// The URL of a table in our test database.
pub(crate) fn post_test_table_url(table_name: &str) -> String {
    format!("{}#{}", postgres_test_url(), table_name)
}

/// The URL to our test `gs://` bucket and directory.
pub(crate) fn gs_url() -> String {
    env::var("GS_TEST_URL").expect("GS_TEST_URL must be set")
}

/// The URL to a subdirectory of `gs_url`.
pub(crate) fn gs_test_dir_url(dir_name: &str) -> String {
    let mut url = gs_url();
    if !url.ends_with('/') {
        url.push('/');
    }
    url.push_str(dir_name);
    url.push('/');
    url
}

/// A BigQuery table name to use for a test, including the project.
pub(crate) fn bq_temp_dataset_name() -> String {
    env::var("BQ_TEST_DATASET").expect("BQ_TEST_DATASET must be set")
}

/// Get our BigQuery test project name.
pub(crate) fn bq_project_id() -> String {
    let ds_name = bq_temp_dataset_name();
    let end = ds_name.find(':').expect("BQ_TEST_DATASET should contain :");
    ds_name[..end].to_owned()
}

/// A BigQuery table to use for a test.
pub(crate) fn bq_temp_dataset() -> String {
    format!("bigquery:{}", bq_temp_dataset_name())
}

/// A BigQuery table to use for a test.
pub(crate) fn bq_test_table(table_name: &str) -> String {
    format!("{}.{}", bq_temp_dataset(), table_name)
}

/// The URL to our test `s3://` bucket and directory.
pub(crate) fn s3_url() -> String {
    env::var("S3_TEST_URL").expect("S3_TEST_URL must be set")
}

/// The URL to a subdirectory of `gs_url`.
pub(crate) fn s3_test_dir_url(dir_name: &str) -> String {
    let mut url = s3_url();
    if !url.ends_with('/') {
        url.push('/');
    }
    url.push_str(dir_name);
    url.push('/');
    url
}

/// The URL of our Redshift test database. Optional because we're not going to
/// keep Redshift running just for unit tests, not at a minimum of $0.25/hour.
pub(crate) fn redshift_test_url() -> Option<String> {
    env::var("REDSHIFT_TEST_URL").ok()
}

/// The URL of a table in our Redshift test database.
pub(crate) fn redshift_test_table_url(table_name: &str) -> Option<String> {
    redshift_test_url().map(|url| format!("{}#{}", url, table_name))
}

#[test]
fn cp_help_flag() {
    let testdir = TestDir::new("dbcrossbar", "cp_help_flag");
    let output = testdir.cmd().args(&["cp", "--help"]).expect_success();
    assert!(output.stdout_str().contains("EXAMPLE LOCATORS:"));
}

/// Given a string containing CSV data, sort all lines except the header
/// alphabetically.
pub(crate) fn normalize_csv_data(csv_data: &str) -> String {
    let mut iter = csv_data.lines();
    let header = iter.next().expect("no CSV headers").to_owned();
    let mut lines = iter.collect::<Vec<_>>();
    lines.sort_unstable();
    format!("{}\n{}\n", header, lines.join("\n"))
}

/// Copy to and from the specified locator, making sure that certain scalar
/// types always produce byte-identical output.
///
/// This is especially important when copying from `locator` to a BigML data
/// set, because BigML has a much more fragile parser than a real database, and
/// it treats `f` and `false` as completely different values. (And it has a
/// somewhat limited date parser.)
///
/// Note that we don't demand exact output for more complex types. Floating
/// point numbers, arrays, GeoJSON, etc., may all be output in multiple ways,
/// depending on the driver. We only try to standardize common types that may
/// cause problems. But we can always standardize more types later.
pub(crate) fn assert_cp_to_exact_csv(test_name: &str, locator: &str) {
    let testdir = TestDir::new("dbcrossbar", test_name);
    let src = testdir.src_path("fixtures/exact_output.csv");
    let schema = testdir.src_path("fixtures/exact_output.sql");
    let gs_temp_dir = gs_test_dir_url(test_name);
    let bq_temp_ds = bq_temp_dataset();
    let s3_temp_dir = s3_test_dir_url(test_name);

    // CSV to locator.
    let output = testdir
        .cmd()
        .args(&[
            "cp",
            "--display-output-locators",
            "--if-exists=overwrite",
            &format!("--temporary={}", gs_temp_dir),
            &format!("--temporary={}", bq_temp_ds),
            &format!("--temporary={}", s3_temp_dir),
            &format!("--schema=postgres-sql:{}", schema.display()),
            &format!("csv:{}", src.display()),
            locator,
        ])
        .tee_output()
        .expect_success();

    // HACK: For drivers which can't read from single files, preserve `locator`,
    // otherwise use the actual destination locator output by dbcrossbar.
    let actual_locator = if locator.starts_with("s3:") || locator.starts_with("gs:") {
        locator
    } else {
        output.stdout_str().trim()
    };

    // Locator to CSV.
    let output = testdir
        .cmd()
        .args(&[
            "cp",
            &format!("--temporary={}", gs_temp_dir),
            &format!("--temporary={}", bq_temp_ds),
            &format!("--schema=postgres-sql:{}", schema.display()),
            &actual_locator,
            "csv:-",
        ])
        .tee_output()
        .expect_success();
    let actual = normalize_csv_data(&output.stdout_str());

    let expected = normalize_csv_data(
        &fs::read_to_string(&src).expect("could not read expected output"),
    );
    assert_diff!(&expected, &actual, ",", 0);
}

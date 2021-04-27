# RedShift

Amazon's [Redshift](https://aws.amazon.com/redshift/) is a cloud-based data warehouse designed to support analytical queries. This driver receives less testing than our BigQuery driver, because the cheapest possible RedShift test system costs over \$100/month. Sponsors are welcome!

## Example locators

These are identical to [PostgreSQL locators](./postgres.html#example-locators), except that `postgres` is replaced by `redshift`:

- `redshift://postgres:$PASSWORD@127.0.0.1:5432/postgres#my_table`

## Configuration & authentication

Authentication is currently handled using the `redshift://user:pass@...` syntax. We may add alternative mechanisms at some point to avoid passing credentials on the command-line.

The following environment variables are required.

- `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`: Set these to your AWS credentials.
- `AWS_SESSION_TOKEN` (optional): This should work, but it hasn't been tested.

The following `--temporary` flag is required:

- `--temporary=s3://$S3_TEMP_BUCKET`: Specify where to stage files for loading or unloading data. The bucket must be in the same region as the Redshift cluster.

[Authentication credentials for `COPY`][copyauth] may be passed using `--to-arg`. For example:

- `--to-arg=iam_role=$ROLE`
- `--to-arg=region=$REGION`

This may require some experimentation.

If you need to generate "-- partner:" SQL comments for an AWS RedShift partner program, you can do it as follows:

- `--to-arg=partner="myapp v1.0"`

[copyauth]: https://docs.aws.amazon.com/redshift/latest/dg/loading-data-access-permissions.html

## Supported features

```txt
{{#include generated/features_redshift.txt}}
```

## Example

```
dbcrossbar cp \
  csv:mytable.csv \
  redshift://myuser:XXXXXX@redshift-cluster-1.YYYYYYY.us-west-2.redshift.amazonaws.com:5439/mydb#mytable  \
  --temporary=s3://mybucket-in-us-west-2/ \
  --if-exists=upsert-on:id \
  --to-arg=credentials="aws_access_key_id=$AWS_ACCESS_KEY_ID;aws_secret_access_key=$AWS_SECRET_ACCESS_KEY;token=$AWS_SESSION_TOKEN"
```

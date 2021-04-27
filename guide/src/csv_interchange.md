# CSV interchange format

Internally, `dbcrossbar` converts all data into CSV streams. For many standard types, all input drivers are required to provide byte-for-byte identical CSV data:

```csv
{{#include ../../dbcrossbar/fixtures/exact_output.csv}}
```

For more complex types such as arrays, structs, JSON, and GeoJSON data, we embed JSON into the CSV file:

```csv
{{#include ../../dbcrossbar/fixtures/many_types.csv}}
```

## Tricks for preparing CSV data

If your input CSV files use an incompatible format, there are several things that might help. If your CSV files are invalid, non-standard, or full of junk, then you may be able to use [`scrubcsv`](https://github.com/faradayio/scrubcsv) or [`xsv`](https://github.com/BurntSushi/xsv) to fix the worst problems.

If you need to clean up your data manually, then you may want to consider using `dbcrossbar` to load your data into BigQuery, and set your columns to type `STRING`. Once this is done, you can parse and normalize your data quickly using SQL queries.

At Faraday, we pass all CSVs on their way into BigQuery through this filter:

```
cat input.csv | \
  tr -d \\0 | \
  scrubcsv \
    --clean-column-names \
    --trim-whitespace \
    --replace-newlines \
    --null \\s\*\(\\s\+\|\\-\|\\\?\|\\\#DIV/0\|\\\#N/A\|\\\#NAME\\\?\|\\\#NULL\!\|\\\#NUM\!\|\\\#REF\!\|\\\#VALUE\!\|0000\\-00\\-00\|n/a\|N/A\|na\|NA\|NaN\|nil\|NIL\|null\|NULL\|unknown\|UNKNOWN\)\\s\* | \
  xsv fixlengths | \
  iconv -c -f UTF8 -t UTF8
```

If you're not using alpine, you might have to call `iconv` with `UTF-8//TRANSLIT`. Here's the unescaped list of values we are treating as null:

* `-`
* `?`
* `#DIV/0`
* `#N/A`
* `#NAME?`
* `#NULL!`
* `#NUM!`
* `#REF!`
* `#VALUE!`
* `0000-00-00`
* `n/a`
* `N/A`
* `na`
* `NA`
* `NaN`
* `nil`
* `NIL`
* `null`
* `NULL`
* `unknown`
* `UNKNOWN`

Sometimes data comes in that lacks a unique id per row, so we add this to the pipeline:

```
gawk 'BEGIN {srand()} NR==1 {print $0 ",random_id"} NR>1{ print $0 "," int(18446744073709551615 * rand() - 9223372036854775807) }'
```

And finally, sometimes we want to specify that certain rows should only be included if they have a value in certain fields. In that case we use `scrubcsv`'s `--drop-row-if-null` option.

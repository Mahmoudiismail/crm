use crm_tool::utils::build_csv_reader_from_reader;

#[test]
fn test_csv_processing_integration_valid() {
    let csv_data = "col1,col2\nval1,val2\nval3,val4";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 2);
    assert_eq!(&record[0], "val1");
    assert_eq!(&record[1], "val2");

    let record2 = iter.next().unwrap().unwrap();
    assert_eq!(record2.len(), 2);
    assert_eq!(&record2[0], "val3");
}

#[test]
fn test_csv_processing_integration_multiline_valid() {
    // A multiline field with properly quoted newlines should be natively read as one field without parsing issues.
    let csv_data = "col1,col2\nval1,\"line1\nline2\nline3\"\nval3,\"escaped\"\"quote\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 2);
    assert_eq!(&record[0], "val1");
    // Verify it parsed as a single string containing actual newlines, not string literals.
    assert_eq!(&record[1], "line1\nline2\nline3");

    let record2 = iter.next().unwrap().unwrap();
    assert_eq!(record2.len(), 2);
    assert_eq!(&record2[0], "val3");
    assert_eq!(&record2[1], "escaped\"quote");
}

#[test]
fn test_csv_processing_integration_invalid_fails_too_many() {
    // strict mode (flexible=false) should fail if column counts don't match (too many)
    let csv_data = "col1,col2\nval1,val2,val3";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

#[test]
fn test_csv_processing_empty_file() {
    // an empty file has no headers.
    let csv_data = "";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let headers_result = rdr.headers();
    assert!(headers_result.is_err() || headers_result.unwrap().is_empty());
}

#[test]
fn test_csv_processing_header_only() {
    // a header-only file should parse headers but have no records.
    let csv_data = "col1,col2,col3\n";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    assert!(!rdr.headers().unwrap().is_empty());
    assert!(rdr.records().next().is_none());
}

#[test]
fn test_csv_processing_integration_invalid_fails_too_few() {
    // strict mode (flexible=false) should fail if column counts don't match (too few)
    let csv_data = "col1,col2,col3\nval1,val2";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

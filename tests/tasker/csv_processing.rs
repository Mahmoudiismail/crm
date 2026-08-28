use crm_tool::utils::build_csv_reader_from_reader;

#[test]
fn test_csv_processing_integration_valid() {
    let csv_data = "col1,col2\nval1,val2";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 2);
    assert_eq!(&record[0], "val1");
    assert_eq!(&record[1], "val2");
}

#[test]
fn test_csv_processing_integration_invalid_fails() {
    // strict mode (flexible=false) should fail if column counts don't match
    let csv_data = "col1,col2\nval1,val2,val3";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

#[test]
fn test_csv_processing_quoted_multiline_valid() {
    let csv_data = "col1,col2\n\"val\n1\",\"val\n2\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 2);
    assert_eq!(&record[0], "val\n1");
    assert_eq!(&record[1], "val\n2");
}

#[test]
fn test_csv_processing_quoted_multiline_invalid_fails() {
    // strict mode (flexible=false) should fail if column counts don't match even if quoted multiline is used
    let csv_data = "col1,col2\n\"val\n1\",\"val\n2\",val3";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

#[test]
fn test_csv_processing_too_few_columns_fails() {
    let csv_data = "col1,col2\nval1";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

#[test]
fn test_csv_processing_too_many_columns_fails() {
    let csv_data = "col1,col2\nval1,val2,val3";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

#[test]
fn test_csv_processing_blank_lines() {
    // Standard strict CSV parsing will fail on a blank line since it has 1 empty column instead of N columns
    // Update: the csv crate ignores empty lines by default when `has_headers(true)` is set or `flexible(false)`. Let's test what happens for a line that isn't totally empty but just spaces.
    let csv_data = "col1,col2\n   \nval1,val2";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();
    let result = iter.next().unwrap();
    assert!(result.is_err());
}

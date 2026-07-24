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

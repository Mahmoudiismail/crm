use crm_tool::utils::build_csv_reader_from_reader;

// A. VALID NORMAL CSV
#[test]
fn test_csv_processing_integration_valid() {
    let csv_data = "id,name,description\n1,John,Normal user\n2,Jane,Admin user";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record1 = iter.next().unwrap().unwrap();
    assert_eq!(record1.len(), 3);
    assert_eq!(&record1[0], "1");
    assert_eq!(&record1[1], "John");
    assert_eq!(&record1[2], "Normal user");

    let record2 = iter.next().unwrap().unwrap();
    assert_eq!(record2.len(), 3);
    assert_eq!(&record2[0], "2");
    assert_eq!(&record2[1], "Jane");
    assert_eq!(&record2[2], "Admin user");
}

// B. VALID QUOTED MULTILINE FIELD
#[test]
fn test_csv_processing_quoted_multiline() {
    let csv_data = "id,name,description\n1,John,\"First line\nSecond line\nThird line\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 3);
    assert_eq!(&record[0], "1");
    assert_eq!(&record[1], "John");
    assert_eq!(&record[2], "First line\nSecond line\nThird line");
}

// C. MULTIPLE MULTILINE RECORDS
#[test]
fn test_csv_processing_multiple_multiline_records() {
    let csv_data = "id,name,description\n1,John,\"A\nB\"\n2,Jane,\"C\nD\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record1 = iter.next().unwrap().unwrap();
    assert_eq!(&record1[0], "1");
    assert_eq!(&record1[2], "A\nB");

    let record2 = iter.next().unwrap().unwrap();
    assert_eq!(&record2[0], "2");
    assert_eq!(&record2[2], "C\nD");
}

// D. MULTILINE FIELD CONTAINING COMMAS
#[test]
fn test_csv_processing_multiline_with_commas() {
    let csv_data = "id,name,description\n1,John,\"Line 1, part A\nLine 2, part B\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 3);
    assert_eq!(&record[2], "Line 1, part A\nLine 2, part B");
}

// E. MULTILINE FIELD CONTAINING QUOTES
#[test]
fn test_csv_processing_multiline_with_quotes() {
    let csv_data = "id,name,description\n1,John,\"He said \"\"Hello\"\"\nAnd then left\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 3);
    assert_eq!(&record[2], "He said \"Hello\"\nAnd then left");
}

// F. UNICODE
#[test]
fn test_csv_processing_unicode() {
    let csv_data = "id,name,description\n1,Ahmed,\"مرحبا\nكيف حالك؟\"";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 3);
    assert_eq!(&record[1], "Ahmed");
    assert_eq!(&record[2], "مرحبا\nكيف حالك؟");
}

// G. EMPTY VALUES
#[test]
fn test_csv_processing_empty_values() {
    let csv_data = "id,name,description\n1,,";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let record = iter.next().unwrap().unwrap();
    assert_eq!(record.len(), 3);
    assert_eq!(&record[0], "1");
    assert_eq!(&record[1], "");
    assert_eq!(&record[2], "");
}

// H. TOO FEW COLUMNS
#[test]
fn test_csv_processing_too_few_columns() {
    let csv_data = "id,name,description\n1,John";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let result = iter.next().unwrap();
    assert!(
        result.is_err(),
        "Expected an error for a record with missing columns in strict mode."
    );
}

// I. TOO MANY COLUMNS
#[test]
fn test_csv_processing_too_many_columns() {
    let csv_data = "id,name,description\n1,John,Desc,Extra";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let result = iter.next().unwrap();
    assert!(
        result.is_err(),
        "Expected an error for a record with extra columns in strict mode."
    );
}

// J. MALFORMED QUOTING
#[test]
fn test_csv_processing_malformed_quoting() {
    // Escaped quote inside unquoted field
    let csv_data = "id,name,description\n1,John,Un\"closed quote";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let mut iter = rdr.records();

    let result = iter.next().unwrap();
    match result {
        Ok(record) => {
            // Since `csv` crate can be forgiving on unquoted fields with quotes inside, let's just make sure the parsed output is not completely corrupted
            assert_eq!(&record[2], "Un\"closed quote");
        }
        Err(_) => {
            // If strict mode catches this, we expect an error.
        }
    }
}

#[test]
fn test_csv_processing_empty_file() {
    let csv_data = "";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    let headers_result = rdr.headers();
    assert!(headers_result.is_err() || headers_result.unwrap().is_empty());
}

#[test]
fn test_csv_processing_header_only() {
    let csv_data = "col1,col2,col3\n";
    let mut rdr = build_csv_reader_from_reader(csv_data.as_bytes());
    assert!(!rdr.headers().unwrap().is_empty());
    assert!(rdr.records().next().is_none());
}

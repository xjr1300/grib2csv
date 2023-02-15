use std::io::BufRead;

use grib2csv::{Boundary, Grib2Csv};

#[test]
#[ignore]
fn test() {
    let input = "fixtures/sample.bin";
    let output = "fixtures/sample.csv";
    let expected = "fixtures/sample.org.csv";
    let grib2 = Grib2Csv::new(input, true).unwrap();
    let boundary = Boundary::default();
    grib2.convert(output, boundary).unwrap();

    use std::fs::File;
    use std::io::BufReader;
    let mut output_reader = BufReader::new(File::open(output).unwrap());
    let expected_reader = BufReader::new(File::open(expected).unwrap());

    for expected_line in expected_reader.lines() {
        let mut output_line = String::new();
        let num_bytes = output_reader.read_line(&mut output_line);
        match num_bytes {
            Ok(0) => assert!(false, "the output csv file can't be read any more"),
            Ok(_) => {
                let expected_line = expected_line.unwrap();
                let output_line = output_line.trim();
                assert_eq!(
                    expected_line, output_line,
                    "expected={expected}, actual={output_line}"
                );
            }
            _ => assert!(false, "the output csv file can't be read any more"),
        }
    }
}

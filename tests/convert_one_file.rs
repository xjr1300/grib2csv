use grib2csv::{Boundary, Grib2Csv};

#[test]
#[ignore]
fn test() {
    let input = "fixtures/sample.bin";
    let output = "fixtures/sample.csv";
    let grib2 = Grib2Csv::new(input, true).unwrap();
    let boundary = Boundary::default();
    grib2.convert(output, boundary).unwrap();
}

use grib2csv::Grib2Csv;

#[test]
#[ignore]
fn test() {
    let input = "fixtures/Z__C_RJTD_20200707073000_SRF_GPV_Ggis1km_Prr60lv_ANAL_grib2.bin";
    let output = "fixtures/Z__C_RJTD_20200707073000_SRF_GPV_Ggis1km_Prr60lv_ANAL_grib2.csv";
    let grib2 = Grib2Csv::new(input).unwrap();
    grib2.convert(output).unwrap();
}

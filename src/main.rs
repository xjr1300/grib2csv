use clap::Parser;
use grib2csv::{BoundaryBuilder, Grib2Csv};

/// コマンドライン引数
#[derive(Debug, Parser)]
struct Args {
    /// 入力GRIB2ファイル
    #[arg(help = "input grib file")]
    input: String,

    /// CSVファイルに出力する格子点の最北端の緯度
    #[arg(
        short,
        long,
        help = "latitude of the northernmost to be output(ex. 36532213)"
    )]
    northernmost: Option<u32>,

    /// CSVファイルに出力する格子点の最南端の緯度
    #[arg(
        short,
        long,
        help = "latitude of the southernmost to be output(ex. 35432213)"
    )]
    southernmost: Option<u32>,

    /// CSVファイルに出力する格子点の最西端の経度
    #[arg(
        short,
        long,
        help = "longitude of the westernmost to be output(ex. 135532213)"
    )]
    westernmost: Option<u32>,

    /// CSVファイルに出力する格子点の最西端の経度
    #[arg(
        short,
        long,
        help = "longitude of the westernmost to be output(ex. 136532213)"
    )]
    easternmost: Option<u32>,

    /// 出力CSVファイル
    #[arg(help = "output csv file")]
    output: String,
}

fn main() {
    let args = Args::parse();
    let converter = Grib2Csv::new(args.input).unwrap();
    let boundary = BoundaryBuilder::default()
        .northernmost(args.northernmost)
        .southernmost(args.southernmost)
        .westernmost(args.westernmost)
        .easternmost(args.easternmost)
        .build();
    converter.convert(args.output, boundary).unwrap();
}

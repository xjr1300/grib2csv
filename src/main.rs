use clap::Parser;
use grib2csv::{BoundaryBuilder, Grib2Csv};

/// コマンドライン引数
#[derive(Parser)]
#[clap(
    name = "grib2csv",
    version = "0.1.4",
    author = "xjr1300.04@gmail.com",
    about = "GRIB2通報式による1kmメッシュ解析雨量または降水短時間予報データを、CSV形式のファイルに変換します。\n\
        欠測値を持つ格子点は、CSVファイルに出力されません。\n\
        格子点を出力する領域を指定する場合、度単位の緯度または経度を1,000,000倍した整数部を指定してください。"
)]
struct Args {
    /// 入力GRIB2ファイル
    #[arg(help = "入力GRIB2ファイルのパス")]
    input: String,

    /// CSVファイルに出力する格子点の最北端の緯度
    #[arg(short, long, help = "格子点を出力する最北端の緯度(例:36000000)")]
    northernmost: Option<u32>,

    /// CSVファイルに出力する格子点の最南端の緯度
    #[arg(short, long, help = "格子点を出力する最南端の緯度(例:35000000)")]
    southernmost: Option<u32>,

    /// CSVファイルに出力する格子点の最西端の経度
    #[arg(short, long, help = "格子点を出力する最西端の経度(例:135000000)")]
    westernmost: Option<u32>,

    /// CSVファイルに出力する格子点の最西端の経度
    #[arg(short, long, help = "格子点を出力する最東端の経度(例:136000000)")]
    easternmost: Option<u32>,

    /// CSVファイルにヘッダを出力しないかを示すフラグ
    #[arg(
        long,
        default_value_t = false,
        help = "CSVファイルにヘッダを出力しない"
    )]
    no_header: bool,

    /// 出力CSVファイル
    #[arg(help = "出力CSVファイルのパス")]
    output: String,
}

fn main() {
    let args = Args::parse();
    let converter = Grib2Csv::new(args.input, !args.no_header).unwrap();
    let boundary = BoundaryBuilder::default()
        .northernmost(args.northernmost)
        .southernmost(args.southernmost)
        .westernmost(args.westernmost)
        .easternmost(args.easternmost)
        .build();
    converter.convert(args.output, boundary).unwrap();
}

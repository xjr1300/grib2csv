use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::str;

use anyhow::anyhow;
use time::{Date, Month, PrimitiveDateTime, Time};

/// 第0節 資料分野: 気象分野
const DOCUMENT_DOMAIN: u8 = 0;
/// 第0節 GRIB版番号
const GRIB_VERSION: u8 = 2;
/// 第1節 GRIBマスター表バーション番号
const GRIB_MASTER_TABLE_VERSION: u8 = 2;
/// 第1節 GRIB地域表バージョン番号
const GRIB_LOCAL_TABLE_VERSION: u8 = 1;
/// 第1節 作成ステータス: 現業プロダクト
const CREATION_STATUS: u8 = 0;
/// 第1節 資料の種類: 解析プロダクト
const DOCUMENT_KIND: u8 = 0;
/// 第3節 格子系定義の出典: 緯度／経度格子（正距円筒図法又はプレートカリー図法）
const GRID_SYSTEM_DEFINITION: u8 = 0;
/// 第3節 資料点数: 2560 * 3360 = 8601600
const NUMBER_OF_POINTS: u32 = 8_601_600;
/// 第3節 格子系定義のテンプレート番号: 緯度・経度格子
const GRID_SYSTEM_DEFINITION_TEMPLATE: u16 = 0;
/// 第3節 地球の形状: GRS80回転楕円体
const EARTH_FIGURE: u8 = 4;
/// 第3節 緯線に沿った格子点数: 2560
const NUMBER_OF_POINT_AT_VERTICAL: u32 = 2_560;
/// 第3節 経線に沿った格子点数: 2560
const NUMBER_OF_POINT_AT_HORIZONTAL: u32 = 3_360;
/// 第3節 原作成領域の基本角
const CREATION_RANGE_ANGLE: u32 = 0;
/// 第3節 最初の格子点の緯度
const NORTHERNMOST_GRID_POINT_LATITUDE: u32 = 47_995_833;
/// 第3節 最初の格子点の経度
const WESTERNMOST_GRID_POINT_LONGITUDE: u32 = 118_006_250;
/// 第3節 最後の格子点の緯度
const SOUTHERNMOST_GRID_POINT_LATITUDE: u32 = 20_004_167;
/// 第3節 最後の格子点の経度
const EASTERNMOST_GRID_POINT_LONGITUDE: u32 = 149_993_750;
/// 第3節 i方向（経線方向）の増分値
const HORIZONTAL_INCREMENT: u32 = 12_500;
/// 第3節 j方向（緯線方向）の増分値
const VERTICAL_INCREMENT: u32 = 8_333;
/// 第3節 走査モード
const SCANNING_MODE: u8 = 0x00;
/// 第5節 資料表現テンプレート番号: ランレングス圧縮
const DOCUMENT_EXPRESSION_TEMPLATE: u16 = 200;
/// 第5節 1データのビット数
const BITS_PER_DATA: u8 = 8;
/// 第5節 レベルの最大値
const MAX_LEVEL: u16 = 98;
/// 第5節 データ代表値の尺度因子
const DATA_VALUE_FACTOR: u8 = 1;

pub struct GRIB2Info {
    /// grib2は世界標準時で日時を記録
    pub date_time: PrimitiveDateTime,
    /// 1データ（レベル値とランレングス値）のビット数
    pub data_per_bits: u8,
    /// 今回の圧縮に用いたレベルの最大値、またはレベルの最大値（どっち？！）
    pub maxv: u16,
    /// レベル値と物理値(mm/h)の対応を格納するコレクション
    pub level_values: HashMap<u16, u16>,
}

impl GRIB2Info {}

/// 第0節 GRIBを読み込んで、"GRIB"が記録されているか確認する。
///
/// ファイル・ポインタが、ファイルの先頭にある必要がある。
fn read_grib(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 4];

    let length = reader.read(&mut buf)?;
    if length != 4 {
        return Err(anyhow!("failed to read a `GRIB`"));
    }
    let s = str::from_utf8(buf.as_slice())?;
    match s {
        "GRIB" => Ok(()),
        _ => Err(anyhow!("failed to read a `GRIB`")),
    }
}

/// ファイルから1バイト読み込み、u8型の値として返却する。
fn read_u8(reader: &mut BufReader<File>) -> anyhow::Result<u8> {
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a u8 value"));
    }

    Ok(u8::from_be_bytes(buf))
}

/// ファイルから2バイト読み込み、u16型の値として返却する。
fn read_u16(reader: &mut BufReader<File>) -> anyhow::Result<u16> {
    let mut buf = [0; 2];
    let length = reader.read(&mut buf)?;
    if length != 2 {
        return Err(anyhow!("failed to read a u16 value"));
    }

    Ok(u16::from_be_bytes(buf))
}

/// ファイルから4バイト読み込み、u32型の値として返却する。
fn read_u32(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    let mut buf = [0; 4];
    let length = reader.read(&mut buf)?;
    if length != 4 {
        return Err(anyhow!("failed to read a u32 value"));
    }

    Ok(u32::from_be_bytes(buf))
}

/// 第0節 資料分野を読み込んで、想定している資料分野であるか確認する。
///
/// ファイル・ポインタが、第0節 GRIBの直後にある必要がある。
fn read_document_domain(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第0節 保留: 2bytes
    reader.seek_relative(2)?;
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a document domain"))?;
    match value {
        DOCUMENT_DOMAIN => Ok(()),
        _ => Err(anyhow!("a document domain is not {DOCUMENT_DOMAIN}")),
    }
}

/// 第0節 GRIB版番号を読み込んで、想定しているGRIB版番号であるか確認する。
///
/// ファイル・ポインタが、第0節 資料分野の直後にある必要がある。
fn read_grib_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a grib version"))?;
    match value {
        GRIB_VERSION => Ok(()),
        _ => Err(anyhow!("a grib version is not {GRIB_VERSION}")),
    }
}

/// 第１節 GRIBマスター表バージョン番号を読み込んで、想定しているGRIBマスター表バージョン番号であるか確認する。
///
/// ファイル・ポインタが、第0節 GRIB版番号の直後にある必要がある。
fn read_grib_master_table_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第0節 GRIB全体の長さ: 8bytes
    // 第1節 節の長さ: 4bytes
    // 第1節 節番号: 1bytes
    // 第1節 作成中枢の識別: 2bytes
    // 第1節 作成副中枢: 2bytes
    reader.seek_relative(17)?;
    let value =
        read_u8(reader).map_err(|_| anyhow!("failed to read a grib master table version"))?;
    match value {
        GRIB_MASTER_TABLE_VERSION => Ok(()),
        _ => Err(anyhow!(
            "a grib master table version is not {GRIB_MASTER_TABLE_VERSION}"
        )),
    }
}

/// 第１節 GRIB地域差バージョン番号を読み込んで、想定しているGRIB地域差バージョン番号であるか確認する。
///
/// ファイル・ポインタが、第1節 GRIBマスター表バージョン番号の直後にある必要がある。
fn read_grib_local_table_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value =
        read_u8(reader).map_err(|_| anyhow!("failed to read a grib local table version"))?;
    match value {
        GRIB_LOCAL_TABLE_VERSION => Ok(()),
        _ => Err(anyhow!(
            "a grib local table version is not {GRIB_LOCAL_TABLE_VERSION}"
        )),
    }
}

/// 第１節 資料の参照日時を読み込んで返却する。
///
/// ファイル・ポインタが、第1節 GRIB地域表バージョン番号の直後にある必要がある。
fn read_reference_date_time(reader: &mut BufReader<File>) -> anyhow::Result<PrimitiveDateTime> {
    // 第1節 参照時刻の意味: 1byte
    reader.seek_relative(1)?;
    // 資料の参照時刻（年）
    let year = read_u16(reader).map_err(|_| anyhow!("failed to read a reference year"))?;
    // 資料の参照時刻（月以降）
    let mut parts = Vec::new();
    for _ in 0..5 {
        let value =
            read_u8(reader).map_err(|_| anyhow!("failed to read for any reference time parts"))?;
        parts.push(value);
    }
    // 日付と時刻を構築
    let month = Month::try_from(parts[0])?;
    let date = Date::from_calendar_date(year as i32, month, parts[1])?;
    let time = Time::from_hms(parts[2], parts[3], parts[4])?;

    Ok(PrimitiveDateTime::new(date, time))
}

/// 第１節 作成ステータスを読み込んで、想定している作成ステータスであるか確認する。
///
/// ファイル・ポインタが、第1節 資料の参照時刻（秒）の直後にある必要がある。
fn read_creation_status(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a creation status"))?;
    match value {
        CREATION_STATUS => Ok(()),
        _ => Err(anyhow!("a creation status is not {CREATION_STATUS}")),
    }
}

/// 第１節 資料の種類を読み込んで、想定している資料の種類であるか確認する。
///
/// ファイルポインタが、第1節 作成ステータスの直後にある必要がある。
fn read_document_kind(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a document kind"))?;
    match value {
        DOCUMENT_KIND => Ok(()),
        _ => Err(anyhow!("a document kind is not {DOCUMENT_KIND}")),
    }
}

/// 第3節 格子系定義の出典を読み込んで、想定している格子系定義の出典であるか確認する。
///
/// ファイル・ポインタが、第1節終了直後（第3節開始）にある必要がある。
fn read_grid_system_definition(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第3節 節の長さ: 4bytes
    // 節番号: 1byte
    reader.seek_relative(5)?;
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a grid system definition"))?;
    match value {
        GRID_SYSTEM_DEFINITION => Ok(()),
        _ => Err(anyhow!(
            "a grid system definition is not {GRID_SYSTEM_DEFINITION}"
        )),
    }
}

/// 第3節 資料点数を読み込んで、返却する。
///
/// ファイル・ポインタが、第3節 節番号の直後にある必要がある。
fn read_number_of_points_in_section3(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    let value =
        read_u32(reader).map_err(|_| anyhow!("failed to read a number of points in section 3"))?;
    match value {
        NUMBER_OF_POINTS => Ok(value),
        _ => Err(anyhow!(
            "a number of points in section 3 is not {NUMBER_OF_POINTS}"
        )),
    }
}

/// 第3節 格子系定義テンプレート番号を読み込んで、想定している格子系定義テンプレート番号であるか確認する。
///
/// ファイル・ポインタが、第3節 資料点数の直後にある必要がある。
fn read_grid_system_definition_template(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第3節 格子点数を定義するリストのオクテット数: 1byte
    // 第3節 格子点数を定義するリストの説明: 1byte
    reader.seek_relative(2)?;
    let value = read_u16(reader)
        .map_err(|_| anyhow!("failed to read a grid system definition template"))?;
    match value {
        GRID_SYSTEM_DEFINITION_TEMPLATE => Ok(()),
        _ => Err(anyhow!(
            "a grid system definition template is not {GRID_SYSTEM_DEFINITION_TEMPLATE}"
        )),
    }
}

/// 第3節 地球の形状を読み込んで、想定している地球の形状であるか確認する。
///
/// ファイル・ポインタが、第3節 格子系定義テンプレート番号の直後にある必要がある。
fn read_earth_figure(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a earth figure"))?;
    match value {
        EARTH_FIGURE => Ok(()),
        _ => Err(anyhow!("a earth figure is not {EARTH_FIGURE}")),
    }
}

/// 第3節 緯線に沿った格子点数を読み込んで、想定している点数であるか確認する。
///
/// ファイル・ポインタが、第3節 地球の形状の直後にある必要がある。
fn read_number_of_points_at_vertical(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第3節 地球球体の半径の尺度因子: 1byte
    // 第3節 地球球体の半径の尺度付き半径: 4bytes
    // 第3節 地球回転楕円体の長軸の尺度因子: 1byte
    // 第3節 地球回転楕円体の長軸の尺度付きの長さ: 4byte
    // 第3節 地球回転楕円体の短軸の尺度因子: 1byte
    // 第3節 地球回転楕円体の短軸の尺度付きの長さ: 4byte
    reader.seek_relative(15)?;
    let value =
        read_u32(reader).map_err(|_| anyhow!("failed to read a number of points at vertical"))?;
    match value {
        NUMBER_OF_POINT_AT_VERTICAL => Ok(()),
        _ => Err(anyhow!(
            "a number of points at vertical is not {NUMBER_OF_POINT_AT_VERTICAL}"
        )),
    }
}

/// 第3節 経線に沿った格子点数を読み込んで、想定している点数であるか確認する。
///
/// ファイル・ポインタが、第3節 緯線に沿った格子点の直後にある必要がある。
fn read_number_of_points_at_horizontal(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value =
        read_u32(reader).map_err(|_| anyhow!("failed to read a number of points at horizontal"))?;
    match value {
        NUMBER_OF_POINT_AT_HORIZONTAL => Ok(()),
        _ => Err(anyhow!(
            "a number of points at horizontal is not {NUMBER_OF_POINT_AT_HORIZONTAL}"
        )),
    }
}

/// 第3節 原作成領域の基本角を読み込んで、想定している角度であるか確認する。
///
/// ファイル・ポインタが、第3節 経線に沿った格子点数の直後にある必要がある。
fn read_creation_range_angle(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u32(reader).map_err(|_| anyhow!("failed to read a creation range angle"))?;
    match value {
        CREATION_RANGE_ANGLE => Ok(()),
        _ => Err(anyhow!(
            "a creation range angle is not {CREATION_RANGE_ANGLE}"
        )),
    }
}

/// 第3節 最初の格子点の緯度を読み込んで、返却する。
///
/// ファイル・ポインタが、第3節 原作成領域の基本角の直後にある必要がある。
fn read_northernmost_grid_point_latitude(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    // 第3節 端点の経度及び緯度並びに方向増分の定義に使われる基本角の細分: 4bytes
    reader.seek_relative(4)?;
    read_u32(reader).map_err(|_| anyhow!("failed to read a northernmost grid point latitude"))
}

/// 第3節 最初の格子点の経度を読み込んで、返却する。
///
/// ファイル・ポインタが、第3節 最初の格子点の緯度の直後にある必要がある。
fn read_westernmost_grid_point_longitude(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a westernmost grid point longitude"))
}

/// 第3節 最後の格子点の緯度を読み込んで、返却する。
///
/// ファイル・ポインタが、第3節 最初の格子点の経度の直後にある必要がある。
fn read_southernmost_grid_point_latitude(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    // 第3節 分解能及び成分フラグ: 1byte
    reader.seek_relative(1)?;
    read_u32(reader).map_err(|_| anyhow!("failed to read a southernmost grid point latitude"))
}

/// 第3節 最後の格子点の経度を読み込んで、返却する。
///
/// ファイル・ポインタが、第3節 最後の格子点の緯度の直後にある必要がある。
fn read_easternmost_grid_point_longitude(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a easternmost grid point longitude"))
}

/// 第3節 i方向（経線方向）の増分を読み込んで、想定している増分か確認する。
///
/// ファイル・ポインタが、第3節 最後の格子点の経度の直後にある必要がある。
fn read_horizontal_increment(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u32(reader).map_err(|_| anyhow!("failed to read a horizontal increment"))?;
    match value {
        HORIZONTAL_INCREMENT => Ok(()),
        _ => Err(anyhow!(
            "a horizontal increment is not {HORIZONTAL_INCREMENT}"
        )),
    }
}

/// 第3節 j方向（緯線方向）の増分を読み込んで、想定している増分か確認する。
///
/// ファイル・ポインタが、第3節 i方向の増分の直後にある必要がある。
fn read_vertical_increment(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u32(reader).map_err(|_| anyhow!("failed to read a vertical increment"))?;
    match value {
        VERTICAL_INCREMENT => Ok(()),
        _ => Err(anyhow!("a vertical increment is not {VERTICAL_INCREMENT}")),
    }
}

/// 第3節 走査モードを読み込んで、想定しているモードか確認する。
///
/// ファイル・ポインタが、第3節 j方向の増分の直後にある必要がある。
fn read_scanning_mode(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a scanning mode"));
    }
    match buf[0] {
        SCANNING_MODE => Ok(()),
        _ => Err(anyhow!("a scanning mode is not {SCANNING_MODE}")),
    }
}

/// 第4節を読み飛ばす。
///
/// ファイルポインタが、第3節 走査モードの直後（第4節の開始）にある必要がある。
fn skip_section4(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第4節 節の長さを読み込み
    let length = read_u32(reader).map_err(|_| anyhow!("failed to read length of section 4"))?;
    // 第4節をスキップ
    reader.seek_relative((length - 4) as i64)?;

    Ok(())
}

/// 第5節 全資料点の数を読み込んで、返却する。
///
/// ファイル・ポインタが、第5節の開始にある必要がある。
fn read_number_of_points_in_section5(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    // 第5節 節の長さ: 4bytes
    // 第5節 節番号: 1byte
    reader.seek_relative(5)?;
    let value =
        read_u32(reader).map_err(|_| anyhow!("failed to read a number of points in section 5"))?;
    match value {
        NUMBER_OF_POINTS => Ok(value),
        _ => Err(anyhow!(
            "a number of points in section 5 is not {NUMBER_OF_POINTS}"
        )),
    }
}

/// 第5節 資料表現テンプレート番号を読み込み、想定している資料表現テンプレート番号であることを確認する。
///
/// ファイル・ポインタが、第5節 全資料点の数の直後にある必要がある。
fn read_document_expression_template(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value =
        read_u16(reader).map_err(|_| anyhow!("failed to read a document expression template"))?;
    match value {
        DOCUMENT_EXPRESSION_TEMPLATE => Ok(()),
        _ => Err(anyhow!(
            "a document expression template is not {DOCUMENT_EXPRESSION_TEMPLATE}"
        )),
    }
}

/// 第5節 1データのビット数を読み込み、想定しているビット数であることを確認する。
///
/// ファイル・ポインタが、第5節 資料表現テンプレート番号の直後にある必要がある。
fn read_bits_per_data(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a bits per data"))?;
    match value {
        BITS_PER_DATA => Ok(()),
        _ => Err(anyhow!("a bits per data is not {BITS_PER_DATA}")),
    }
}

/// 第5節 今回の圧縮に用いたレベルの最大値を読み込み、返却する。
///
/// ファイル・ポインタが、第5節 1データのビット数の直後にある必要がある。
fn read_max_level_of_this_time(reader: &mut BufReader<File>) -> anyhow::Result<u16> {
    read_u16(reader).map_err(|_| anyhow!("failed to read a max level of this time"))
}

/// 第5節 レベルの最大値を読み込み、返却する。
///
/// ファイル・ポインタが、第5節 今回の圧縮に用いたレベルの最大値の直後にある必要がある。
fn read_max_level(reader: &mut BufReader<File>) -> anyhow::Result<u16> {
    read_u16(reader).map_err(|_| anyhow!("failed to read a max level"))
}

/// 第5節 データ代表値の尺度因子を読み込み、想定している尺度因子であることを確認する。
///
/// ファイル・ポインタが、第5節 レベルの最大値の直後にある必要がある。
fn read_data_value_factor(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a data value factor"))?;
    match value {
        DATA_VALUE_FACTOR => Ok(()),
        _ => Err(anyhow!("a data value factor is not {DATA_VALUE_FACTOR}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    const SAMPLE_FILE: &'static str = "fixtures/20200707000000_grib2.bin";
    const SAMPLE_MAX_LEVEL_THIS_TIME: u16 = 73;

    #[test]
    fn can_read_grib_file() {
        let mut reader = BufReader::new(File::open(SAMPLE_FILE).unwrap());
        // GRIBを読み込めるか確認
        assert!(read_grib(&mut reader).is_ok(), "failed to read a `GRIB`");
        // 資料分野が正しいか確認
        assert!(
            read_document_domain(&mut reader).is_ok(),
            "failed to read a document domain"
        );
        // GRIB版番号が正しいか確認
        assert!(
            read_grib_version(&mut reader).is_ok(),
            "failed to read a grib version"
        );
        // GRIBマスター表バージョン番号が正しいか確認
        assert!(
            read_grib_master_table_version(&mut reader).is_ok(),
            "failed to read a grib master table version"
        );
        // GRIB地域表バージョン番号が正しいか確認
        assert!(
            read_grib_local_table_version(&mut reader).is_ok(),
            "failed to read a grib local table version"
        );
        // 資料の参照日時が正しいか確認
        let dt = read_reference_date_time(&mut reader);
        assert!(dt.is_ok(), "failed to read a reference date time");
        assert_eq!(dt.unwrap(), datetime!(2020-07-07 00:00:00));
        // 作成ステータスが正しいか確認
        assert!(
            read_creation_status(&mut reader).is_ok(),
            "failed to read a creation status"
        );
        // 資料の種類が正しいか確認
        assert!(
            read_document_kind(&mut reader).is_ok(),
            "failed to read a document kind"
        );
        // 格子系定義の出典
        assert!(
            read_grid_system_definition(&mut reader).is_ok(),
            "failed to read a grid system definition"
        );
        // 資料点数
        let number_of_points = read_number_of_points_in_section3(&mut reader);
        assert!(number_of_points.is_ok());
        assert_eq!(number_of_points.unwrap(), NUMBER_OF_POINTS);
        // 格子系定義テンプレート番号
        assert!(
            read_grid_system_definition_template(&mut reader).is_ok(),
            "failed to read a grid system definition template"
        );
        // 地球の形状
        assert!(
            read_earth_figure(&mut reader).is_ok(),
            "failed to read a earth figure"
        );
        // 緯線に沿った格子点数
        assert!(
            read_number_of_points_at_vertical(&mut reader).is_ok(),
            "failed to read a number of points at vertical"
        );
        // 経線に沿った格子点数。
        assert!(
            read_number_of_points_at_horizontal(&mut reader).is_ok(),
            "failed to read a number of points at horizontal"
        );
        // 原作成領域の基本角
        assert!(
            read_creation_range_angle(&mut reader).is_ok(),
            "failed to read a creation range angle"
        );
        // 最初の格子点の緯度
        let number_of_points = read_northernmost_grid_point_latitude(&mut reader);
        assert!(number_of_points.is_ok());
        assert_eq!(
            number_of_points.unwrap(),
            NORTHERNMOST_GRID_POINT_LATITUDE,
            "a northernmost grid point latitude is not {NORTHERNMOST_GRID_POINT_LATITUDE}"
        );
        // 最初の格子点の経度
        let number_of_points = read_westernmost_grid_point_longitude(&mut reader);
        assert!(number_of_points.is_ok());
        assert_eq!(
            number_of_points.unwrap(),
            WESTERNMOST_GRID_POINT_LONGITUDE,
            "a westernmost grid point longitude is not {WESTERNMOST_GRID_POINT_LONGITUDE}"
        );
        // 最後の格子点の緯度
        let number_of_points = read_southernmost_grid_point_latitude(&mut reader);
        assert!(number_of_points.is_ok());
        assert_eq!(
            number_of_points.unwrap(),
            SOUTHERNMOST_GRID_POINT_LATITUDE,
            "a southernmost grid point latitude is not {SOUTHERNMOST_GRID_POINT_LATITUDE}"
        );
        // 最後の格子点の経度
        let number_of_points = read_easternmost_grid_point_longitude(&mut reader);
        assert!(number_of_points.is_ok());
        assert_eq!(
            number_of_points.unwrap(),
            EASTERNMOST_GRID_POINT_LONGITUDE,
            "a easternmost grid point longitude is not {EASTERNMOST_GRID_POINT_LONGITUDE}"
        );
        // i方向の増分
        assert!(read_horizontal_increment(&mut reader).is_ok());
        // j方向の増分
        assert!(read_vertical_increment(&mut reader).is_ok());
        // 走査モード
        assert!(read_scanning_mode(&mut reader).is_ok());
        // 第4節を読み飛ばす
        assert!(skip_section4(&mut reader).is_ok());
        // 第5節の全資料点の数
        assert!(read_number_of_points_in_section5(&mut reader).is_ok());
        // 資料表現テンプレート番号
        assert!(read_document_expression_template(&mut reader).is_ok());
        // 1データのビット数
        assert!(read_bits_per_data(&mut reader).is_ok());
        // 今回の圧縮に用いたレベルの最大値
        let max_level = read_max_level_of_this_time(&mut reader);
        assert!(max_level.is_ok());
        assert_eq!(max_level.unwrap(), SAMPLE_MAX_LEVEL_THIS_TIME);
        // レベルの最大値
        let max_level = read_max_level(&mut reader);
        assert!(max_level.is_ok());
        assert_eq!(max_level.unwrap(), MAX_LEVEL);
        // データ代表値の尺度因子
        assert!(read_data_value_factor(&mut reader).is_ok());
    }
}

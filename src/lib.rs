use std::fs::File;
use std::io::{BufReader, Read};
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
/// 第3節 走査モード
const SCANNING_MODE: u8 = 0x00;
/// 第5節 資料表現テンプレート番号: ランレングス圧縮
const DOCUMENT_EXPRESSION_TEMPLATE: u16 = 200;
/// 第5節 1データのビット数
const BITS_PER_DATA: u8 = 8;
/// 第5節 データ代表値の尺度因子
const DATA_VALUE_FACTOR: u8 = 1;

pub struct GRIB2Info {
    /// grib2は世界標準時で日時を記録
    pub date_time: PrimitiveDateTime,
    /// 1データ（レベル値とランレングス値）のビット数
    pub data_per_bits: u8,
    /// 今回の圧縮に用いたレベルの最大値、またはレベルの最大値（どっち？！）
    pub maxv: u16,
    /// 物理値(mm/h)の対応を格納するコレクション
    /// レベルnの物理値は、コレクションのn-1の位置に記録
    pub level_values: Vec<u16>,
}

impl GRIB2Info {}

/// ファイルから1バイト読み込み、u8型の値として返却する。
fn read_u8(reader: &mut BufReader<File>) -> anyhow::Result<u8> {
    let mut buf = [0; 1];
    let size = reader.read(&mut buf)?;
    if size != 1 {
        return Err(anyhow!("failed to read a u8 value"));
    }

    Ok(u8::from_be_bytes(buf))
}

/// ファイルから2バイト読み込み、u16型の値として返却する。
fn read_u16(reader: &mut BufReader<File>) -> anyhow::Result<u16> {
    let mut buf = [0; 2];
    let size = reader.read(&mut buf)?;
    if size != 2 {
        return Err(anyhow!("failed to read a u16 value"));
    }

    Ok(u16::from_be_bytes(buf))
}

/// ファイルから4バイト読み込み、u32型の値として返却する。
fn read_u32(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    let mut buf = [0; 4];
    let size = reader.read(&mut buf)?;
    if size != 4 {
        return Err(anyhow!("failed to read a u32 value"));
    }

    Ok(u32::from_be_bytes(buf))
}

/// 第0節を読み込み、内容を確認する。
///
/// ファイル・ポインタが、ファイルの先頭にあることを想定している。
/// 関数終了後、ファイル・ポインタは第1節の開始位置に移動する。
pub fn read_section0(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // GRIB
    read_section0_grib(reader)?;
    // 保留: 2bytes
    reader.seek_relative(2)?;
    // 資料分野
    read_section0_document_domain(reader)?;
    // GRIB反番号
    read_section0_grib_version(reader)?;

    // GRIB報全体の長さ
    reader.seek_relative(8).map_err(|e| e.into())
}

/// 第0節 GRIBを読み込んで、"GRIB"が記録されているか確認する。
fn read_section0_grib(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 4];

    let size = reader.read(&mut buf)?;
    if size != 4 {
        return Err(anyhow!("failed to read a `GRIB`"));
    }
    let s = str::from_utf8(buf.as_slice())?;
    match s {
        "GRIB" => Ok(()),
        _ => Err(anyhow!("failed to read a `GRIB`")),
    }
}

/// 第0節 資料分野を読み込んで、想定している資料分野であるか確認する。
fn read_section0_document_domain(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a document domain"))?;
    match value {
        DOCUMENT_DOMAIN => Ok(()),
        _ => Err(anyhow!("a document domain is not {DOCUMENT_DOMAIN}")),
    }
}

/// 第0節 GRIB版番号を読み込んで、想定しているGRIB版番号であるか確認する。
fn read_section0_grib_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a grib version"))?;
    match value {
        GRIB_VERSION => Ok(()),
        _ => Err(anyhow!("a grib version is not {GRIB_VERSION}")),
    }
}

/// 第1節情報
pub struct Section1 {
    /// 資料の参照時刻（日時）
    pub referenced_at: PrimitiveDateTime,
}

/// 第1節を読み込んで、第1節の情報を返却する。
///
/// ファイルポインタが、第1節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第3節の開始位置に移動する。
/// なお、実装時点で、第2節は省略されている。
pub fn read_section1(reader: &mut BufReader<File>) -> anyhow::Result<Section1> {
    // 節の長さ: 4bytes
    reader.seek_relative(4)?;
    // 節番号
    let section_number =
        read_u8(reader).map_err(|_| anyhow!("failed to read section number at section 1"))?;
    if section_number != 1 {
        return Err(anyhow!("section number is miss match in section 1"));
    }
    // 作成中枢の識別: 2bytes
    // 作成副中枢: 2bytes
    reader.seek_relative(4)?;
    // GRIBマスター表バージョン番号
    read_section1_grib_master_table_version(reader)?;
    // GRIB地域表バージョン番号
    read_section1_grib_local_table_version(reader)?;
    // 参照時刻の意味: 1byte
    reader.seek_relative(1)?;
    // 資料の参照時刻（日時）
    let referenced_at = read_section1_referenced_at(reader)?;
    // 作成ステータス
    read_section1_creation_status(reader)?;
    // 資料の種類
    read_section1_document_kind(reader)?;

    Ok(Section1 { referenced_at })
}

/// 第１節 GRIBマスター表バージョン番号を読み込んで、想定しているGRIBマスター表バージョン番号であるか確認する。
fn read_section1_grib_master_table_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
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
fn read_section1_grib_local_table_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
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
fn read_section1_referenced_at(reader: &mut BufReader<File>) -> anyhow::Result<PrimitiveDateTime> {
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
fn read_section1_creation_status(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a creation status"))?;
    match value {
        CREATION_STATUS => Ok(()),
        _ => Err(anyhow!("a creation status is not {CREATION_STATUS}")),
    }
}

/// 第１節 資料の種類を読み込んで、想定している資料の種類であるか確認する。
fn read_section1_document_kind(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a document kind"))?;
    match value {
        DOCUMENT_KIND => Ok(()),
        _ => Err(anyhow!("a document kind is not {DOCUMENT_KIND}")),
    }
}

/// 第3節情報
pub struct Section3 {
    /// 資料点数
    pub number_of_points: u32,
    /// 最初（最も左上）の格子点の緯度（10^6度単位）
    pub northernmost: u32,
    /// 最初（最も左上）の格子点の経度（10^6度単位）
    pub westernmost: u32,
    /// 最後（最も右下）の格子点の緯度（10^6度単位）
    pub southernmost: u32,
    /// 最後（最も右下）の格子点の経度（10^6度単位）
    pub easternmost: u32,
    /// i方向（経線方向）の増分（10^6度単位）
    pub horizontal_increment: u32,
    /// j方向（緯線方向）の増分（10^6度単位）
    pub vertical_increment: u32,
}

/// 第3節を読み込んで、第3節の情報を返却する。
///
/// ファイルポインタが、第3節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第4節の開始位置に移動する。
pub fn read_section3(reader: &mut BufReader<File>) -> anyhow::Result<Section3> {
    // 節の長さ: 4bytes
    reader.seek_relative(4)?;
    // 節番号
    let section_number =
        read_u8(reader).map_err(|_| anyhow!("failed to read section number at section 3"))?;
    if section_number != 3 {
        return Err(anyhow!("section number is miss match in section 3"));
    }
    // 格子系定義の出典
    read_section3_grid_system_definition(reader)?;
    // 資料点数
    let number_of_points = read_section3_number_of_points(reader)?;
    // 格子点を定義するリストのオクテット数: 1byte
    // 格子点を定義するリストの説明: 1byte
    reader.seek_relative(2)?;
    // 格子系定義テンプレート番号
    read_section3_grid_system_definition_template(reader)?;
    // 地球の形状
    read_section3_earth_figure(reader)?;
    // 地球球体の半径の尺度因子: 1byte
    // 地球球体の半径の尺度付き半径: 4bytes
    // 地球回転楕円体の長軸の尺度因子: 1byte
    // 地球回転楕円体の長軸の尺度付きの長さ: 4byte
    // 地球回転楕円体の短軸の尺度因子: 1byte
    // 地球回転楕円体の短軸の尺度付きの長さ: 4byte
    reader.seek_relative(15)?;
    // 緯線に沿った格子点数
    read_section3_number_of_points_at_vertical(reader)?;
    // 経線に沿った格子点数
    read_section3_number_of_points_at_horizontal(reader)?;
    // 原作成領域の基本角
    read_section3_creation_range_angle(reader)?;
    // 端点の経度及び緯度並びに方向増分の定義に使われる基本角の細分: 4bytes
    reader.seek_relative(4)?;
    // 最初の格子点の緯度
    let northernmost = read_section3_northernmost_degree(reader)?;
    // 最初の格子点の経度
    let westernmost = read_section3_westernmost_degree(reader)?;
    // 分解能及び成分フラグ: 1byte
    reader.seek_relative(1)?;
    // 最後の格子点の緯度
    let southernmost = read_section3_southernmost_degree(reader)?;
    // 最後の格子点の経度
    let easternmost = read_section3_easternmost_degree(reader)?;
    // i方向の増分
    let horizontal_increment = read_section3_horizontal_increment(reader)?;
    // j方向の増分
    let vertical_increment = read_section3_vertical_increment(reader)?;
    // 走査モード
    read_section3_scanning_mode(reader)?;

    Ok(Section3 {
        number_of_points,
        northernmost,
        westernmost,
        southernmost,
        easternmost,
        horizontal_increment,
        vertical_increment,
    })
}

/// 第3節 格子系定義の出典を読み込んで、想定している格子系定義の出典であるか確認する。
fn read_section3_grid_system_definition(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a grid system definition"))?;
    match value {
        GRID_SYSTEM_DEFINITION => Ok(()),
        _ => Err(anyhow!(
            "a grid system definition is not {GRID_SYSTEM_DEFINITION}"
        )),
    }
}

/// 第3節 資料点数を読み込んで、返却する。
fn read_section3_number_of_points(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a number of points in section 3"))
}

/// 第3節 格子系定義テンプレート番号を読み込んで、想定している格子系定義テンプレート番号であるか確認する。
fn read_section3_grid_system_definition_template(
    reader: &mut BufReader<File>,
) -> anyhow::Result<()> {
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
fn read_section3_earth_figure(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a earth figure"))?;
    match value {
        EARTH_FIGURE => Ok(()),
        _ => Err(anyhow!("a earth figure is not {EARTH_FIGURE}")),
    }
}

/// 第3節 緯線に沿った格子点数を読み込んで、想定している点数であるか確認する。
fn read_section3_number_of_points_at_vertical(reader: &mut BufReader<File>) -> anyhow::Result<()> {
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
fn read_section3_number_of_points_at_horizontal(
    reader: &mut BufReader<File>,
) -> anyhow::Result<()> {
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
fn read_section3_creation_range_angle(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u32(reader).map_err(|_| anyhow!("failed to read a creation range angle"))?;
    match value {
        CREATION_RANGE_ANGLE => Ok(()),
        _ => Err(anyhow!(
            "a creation range angle is not {CREATION_RANGE_ANGLE}"
        )),
    }
}

/// 第3節 最初の格子点の緯度を読み込んで、返却する。
fn read_section3_northernmost_degree(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a northernmost grid point latitude"))
}

/// 第3節 最初の格子点の経度を読み込んで、返却する。
fn read_section3_westernmost_degree(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a westernmost grid point longitude"))
}

/// 第3節 最後の格子点の緯度を読み込んで、返却する。
fn read_section3_southernmost_degree(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a southernmost grid point latitude"))
}

/// 第3節 最後の格子点の経度を読み込んで、返却する。
fn read_section3_easternmost_degree(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a easternmost grid point longitude"))
}

/// 第3節 i方向（経線方向）の増分を読み込んで、想定している増分か確認する。
fn read_section3_horizontal_increment(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a horizontal increment"))
}

/// 第3節 j方向（緯線方向）の増分を読み込んで、想定している増分か確認する。
fn read_section3_vertical_increment(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a vertical increment"))
}

/// 第3節 走査モードを読み込んで、想定しているモードか確認する。
fn read_section3_scanning_mode(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a scanning mode"))?;
    match value {
        SCANNING_MODE => Ok(()),
        _ => Err(anyhow!("a scanning mode is not {SCANNING_MODE}")),
    }
}

/// 第4節を読み混んで確認する。
///
/// ファイルポインタが、第4節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第5節の開始位置に移動する。
pub fn read_section4(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第4節 節の長さを読み込み
    let length = read_u32(reader).map_err(|_| anyhow!("failed to read length of section 4"))?;
    // 節番号
    let section_number =
        read_u8(reader).map_err(|_| anyhow!("failed to read section number at section 4"))?;
    if section_number != 4 {
        return Err(anyhow!("section number is miss match in section 4"));
    }

    // テンプレート直後の座標値の数以降をスキップ
    reader
        .seek_relative((length - (4 + 1)) as i64)
        .map_err(|e| e.into())
}

/// 第5節情報
pub struct Section5 {
    /// 全資料点の数
    pub number_of_points: u32,
    /// 1データのビット数
    pub bits_per_data: u8,
    /// 今回の圧縮に落ちいたレベルの最大値
    pub max_level_at_file: u16,
    /// レベルの最大値
    pub max_level: u16,
    /// レベルmに対応するデータ代表値
    /// レベル値と物理値(mm/h)の対応を格納するコレクション
    pub level_values: Vec<u16>,
}

/// 第5節を読み込んで、第3節の情報を返却する。
///
/// ファイルポインタが、第5節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第6節の開始位置に移動する。
pub fn read_section5(reader: &mut BufReader<File>) -> anyhow::Result<Section5> {
    // 節の長さ
    let length = read_u32(reader).map_err(|_| anyhow!("failed to read length of section 5"))?;
    // 節番号
    let section_number =
        read_u8(reader).map_err(|_| anyhow!("failed to read section number at section 5"))?;
    if section_number != 5 {
        return Err(anyhow!("section number is miss match in section 5"));
    }
    // 全資料点の数
    let number_of_points = read_section5_number_of_points(reader)?;
    // 資料表現テンプレート番号
    read_section5_document_expression_template(reader)?;
    // 1データのビット数
    let bits_per_data = read_section5_bits_per_data(reader)?;
    // 今回の圧縮に用いたレベルの最大値
    let max_level_at_file = read_section5_max_level_of_this_time(reader)?;
    // レベルの私大値
    let max_level = read_section5_max_level(reader)?;
    // データ代表値の尺度因子
    read_section5_data_value_factor(reader)?;
    // レベルmに対応するデータ代表値
    let remaining_length = (length - (4 + 1 + 4 + 2 + 1 + 2 + 2 + 1)) as u16;
    let number_of_levels = remaining_length / 2;
    let mut level_values = Vec::new();
    for _ in 0..number_of_levels {
        level_values.push(read_u16(reader).map_err(|_| anyhow!("failed to read a level value"))?);
    }

    Ok(Section5 {
        number_of_points,
        bits_per_data,
        max_level_at_file,
        max_level,
        level_values,
    })
}

/// 第5節 全資料点の数を読み込んで、返却する。
fn read_section5_number_of_points(reader: &mut BufReader<File>) -> anyhow::Result<u32> {
    // 第5節 節番号: 1byte
    read_u32(reader).map_err(|_| anyhow!("failed to read a number of points in section 5"))
}

/// 第5節 資料表現テンプレート番号を読み込み、想定している資料表現テンプレート番号であることを確認する。
fn read_section5_document_expression_template(reader: &mut BufReader<File>) -> anyhow::Result<()> {
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
fn read_section5_bits_per_data(reader: &mut BufReader<File>) -> anyhow::Result<u8> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a bits per data"))?;
    match value {
        BITS_PER_DATA => Ok(value),
        _ => Err(anyhow!("a bits per data is not {BITS_PER_DATA}")),
    }
}

/// 第5節 今回の圧縮に用いたレベルの最大値を読み込み、返却する。
fn read_section5_max_level_of_this_time(reader: &mut BufReader<File>) -> anyhow::Result<u16> {
    read_u16(reader).map_err(|_| anyhow!("failed to read a max level of this time"))
}

/// 第5節 レベルの最大値を読み込み、返却する。
fn read_section5_max_level(reader: &mut BufReader<File>) -> anyhow::Result<u16> {
    read_u16(reader).map_err(|_| anyhow!("failed to read a max level"))
}

/// 第5節 データ代表値の尺度因子を読み込み、想定している尺度因子であることを確認する。
fn read_section5_data_value_factor(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a data value factor"))?;
    match value {
        DATA_VALUE_FACTOR => Ok(()),
        _ => Err(anyhow!("a data value factor is not {DATA_VALUE_FACTOR}")),
    }
}

/// 第6節を読み込んで、確認する。
///
/// ファイルポインタが、第5節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第6節の開始位置に移動する。
pub fn read_section6(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 節の長さ: 4bytes
    reader.seek_relative(4)?;
    // 節番号
    let section_number =
        read_u8(reader).map_err(|_| anyhow!("failed to read section number at section 6"))?;
    if section_number != 6 {
        return Err(anyhow!("section number is miss match in section 6"));
    }

    // ビットマップ指示符
    reader.seek_relative(1).map_err(|e| e.into())
}

/// 第7節を読み込んで、確認する。
pub fn read_section7(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 節の長さ: 4bytes
    let length = read_u32(reader).map_err(|_| anyhow!("failed to read length of section 7"))?;
    // 節番号
    let section_number =
        read_u8(reader).map_err(|_| anyhow!("failed to read section number at section 7"))?;
    if section_number != 7 {
        return Err(anyhow!("section number is miss match in section 7"));
    }

    // TODO: ランレングス圧縮オクテット列をスキップ
    reader
        .seek_relative((length - (4 + 1)) as i64)
        .map_err(|e| e.into())
}

/// 第8節を読み込んで、確認する。
pub fn read_section8(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 4];
    let size = reader
        .read(&mut buf)
        .map_err(|_| anyhow!("failed to read a `7777`"))?;
    if size != 4 {
        return Err(anyhow!("failed to read a `7777`"));
    }
    let s = str::from_utf8(buf.as_slice())?;

    match s {
        "7777" => Ok(()),
        _ => Err(anyhow!("failed to read a `7777`")),
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
        // 第0節を読み込み
        assert!(read_section0(&mut reader).is_ok());
        // 第1節を読み込み
        let section1 = read_section1(&mut reader).unwrap();
        assert_eq!(section1.referenced_at, datetime!(2020-07-07 00:00:00));
        // 第3節を読み込み
        let section3 = read_section3(&mut reader).unwrap();
        assert_eq!(section3.number_of_points, 2560 * 3360);
        assert_eq!(section3.northernmost, 47995833);
        assert_eq!(section3.westernmost, 118006250);
        assert_eq!(section3.southernmost, 20004167);
        assert_eq!(section3.easternmost, 149993750);
        assert_eq!(section3.horizontal_increment, 12500);
        assert_eq!(section3.vertical_increment, 8333);
        // 第4節を読み飛ばす
        assert!(read_section4(&mut reader).is_ok());
        // 第5節を読み込み
        let section5 = read_section5(&mut reader).unwrap();
        assert_eq!(section5.number_of_points, 8601600);
        assert_eq!(section5.bits_per_data, 8);
        assert_eq!(section5.max_level_at_file, SAMPLE_MAX_LEVEL_THIS_TIME);
        assert_eq!(section5.max_level, 98);
        assert!(section5.max_level_at_file <= section5.max_level);
        assert_eq!(section5.level_values, sample_level_values());
        // 第6節を読み込み
        assert!(read_section6(&mut reader).is_ok());
        // 第7節を読み込み
        assert!(read_section7(&mut reader).is_ok());
        // 第8節を読み込み
        assert!(read_section8(&mut reader).is_ok());
    }

    fn sample_level_values() -> Vec<u16> {
        vec![
            0, 4, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 180,
            190, 200, 210, 220, 230, 240, 250, 260, 270, 280, 290, 300, 310, 320, 330, 340, 350,
            360, 370, 380, 390, 400, 410, 420, 430, 440, 450, 460, 470, 480, 490, 500, 510, 520,
            530, 540, 550, 560, 570, 580, 590, 600, 610, 620, 630, 640, 650, 660, 670, 680, 690,
            700, 710, 720, 730, 740, 750, 760, 770, 800, 850, 900, 950, 1000, 1050, 1100, 1150,
            1200, 1250, 1300, 1400, 1500, 1600, 1700, 1800, 1900, 2000, 2550,
        ]
    }
}

use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::str;

use anyhow::anyhow;
use time::{Date, Month, PrimitiveDateTime, Time};

type FileReader = BufReader<File>;
type FileWriter = BufWriter<File>;

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

/// GRIB2ファイル・コンバーター
pub struct Grib2Csv {
    reader: RefCell<FileReader>,
    section3: Section3,
    section5: Section5,
    with_header: bool,
}

#[derive(Default)]
pub struct Boundary {
    northernmost: Option<u32>,
    southernmost: Option<u32>,
    westernmost: Option<u32>,
    easternmost: Option<u32>,
}

impl Boundary {
    fn contains(&self, longitude: u32, latitude: u32) -> bool {
        if let Some(northernmost) = self.northernmost {
            if northernmost < latitude {
                return false;
            }
        }
        if let Some(southernmost) = self.southernmost {
            if latitude < southernmost {
                return false;
            }
        }
        if let Some(westernmost) = self.westernmost {
            if longitude < westernmost {
                return false;
            }
        }
        if let Some(easternmost) = self.easternmost {
            if easternmost < longitude {
                return false;
            }
        }

        true
    }
}

#[derive(Default)]
pub struct BoundaryBuilder {
    northernmost: Option<u32>,
    southernmost: Option<u32>,
    westernmost: Option<u32>,
    easternmost: Option<u32>,
}

impl BoundaryBuilder {
    pub fn northernmost(mut self, degree: Option<u32>) -> Self {
        self.northernmost = degree;

        self
    }

    pub fn southernmost(mut self, degree: Option<u32>) -> Self {
        self.southernmost = degree;

        self
    }

    pub fn westernmost(mut self, degree: Option<u32>) -> Self {
        self.westernmost = degree;

        self
    }

    pub fn easternmost(mut self, degree: Option<u32>) -> Self {
        self.easternmost = degree;

        self
    }

    pub fn build(self) -> Boundary {
        Boundary {
            northernmost: self.northernmost,
            southernmost: self.southernmost,
            westernmost: self.westernmost,
            easternmost: self.easternmost,
        }
    }
}

impl Grib2Csv {
    /// コンストラクタ
    ///
    /// # 引数
    ///
    /// * `path` - grib2ファイルのパス。
    /// * `with_header` - ヘッダ出力フラグ。
    ///
    /// # 戻り値
    ///
    /// GRIB2Infoインスタンス。
    pub fn new<P: AsRef<Path>>(path: P, with_header: bool) -> anyhow::Result<Self> {
        let mut reader = BufReader::new(File::open(path.as_ref())?);
        // 第0節を読み込み
        read_section0(&mut reader)?;
        // 第1節を読み込み
        read_section1(&mut reader)?;
        // 第3節を読み込み
        let section3 = read_section3(&mut reader)?;
        // 第4節を読み込み
        read_section4(&mut reader)?;
        // 第5節を読み込み
        let section5 = read_section5(&mut reader)?;
        if section3.number_of_points != section5.number_of_points {
            return Err(anyhow!(
                "the number of points is different (section3:{}, section5:{})",
                section3.number_of_points,
                section5.number_of_points
            ));
        }
        // 第6節を読み込み
        read_section6(&mut reader)?;

        Ok(Self {
            reader: RefCell::new(reader),
            section3,
            section5,
            with_header,
        })
    }

    /// GRIB2ファイルの第7節を読み込んで、データをCSV形式のファイルに出力する。
    ///
    /// GRIB2ファイルを正確に読み込みできたか確認するために、処理の最後で第8節を読み込み、
    /// "7777"を読み込めるか確認する。
    ///
    /// # 引数
    ///
    /// * `path` - 変換後のデータを記録するCSV形式のファイルのパス。
    /// * `boundary` - CSVファイルに出力する格子点の境界。
    pub fn convert<P: AsRef<Path>>(&self, path: P, boundary: Boundary) -> anyhow::Result<()> {
        // CSVファイルを作成して、ヘッダを出力
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        // ヘッダ出力
        if self.with_header {
            writeln!(writer, "longitude,latitude,value")?;
        }

        // 第7節を読み込み、ランレングス圧縮オクテット列の直前まで読み込み
        let mut reader = self.reader.borrow_mut();
        // 節の長さ: 4bytes
        let section_bytes = read_u32(&mut reader)?;
        // 節番号
        let section_number = read_u8(&mut reader)?;
        if section_number != 7 {
            return Err(anyhow!(
                "failed to read for the wrong section number(expected:7, read:{section_number}"
            ));
        }
        // ランレングス圧縮オクテット列のバイト数を計算
        // ランレングス圧縮を展開するための情報を精霊
        let maxv = self.section5.max_level_at_file;
        let nbit = self.section5.bits_per_data;
        let lngu = 2u16.pow(nbit as u32) - 1 - maxv;
        // ランレングス圧縮オクテットを展開して、CSVファイルに書き込み
        let mut run_length = Vec::new();
        let mut longitude = self.section3.westernmost;
        let mut latitude = self.section3.northernmost;
        let mut number_of_read = 0u32; // 読み込んだ格子点の数
        for _ in 0..section_bytes - (4 + 1) {
            let value = (read_u8(&mut reader)?) as u16;
            if value <= maxv && !run_length.is_empty() {
                // ランレングス符号を展開
                let (level, count) = expand_run_length(&run_length, maxv, lngu);
                number_of_read += count;
                // レベル値を物理値に変換して書き込み
                self.output_values(
                    &mut writer,
                    level,
                    count,
                    &mut longitude,
                    &mut latitude,
                    &boundary,
                )?;
                run_length.clear();
            }
            run_length.push(value);
        }
        if !run_length.is_empty() {
            let (level, count) = expand_run_length(&run_length, maxv, lngu);
            number_of_read += count;
            self.output_values(
                &mut writer,
                level,
                count,
                &mut longitude,
                &mut latitude,
                &boundary,
            )?;
        }
        writer.flush()?;
        if number_of_read != self.section3.number_of_points {
            return Err(anyhow!(
                "failed to read points (expected:{}, read:{})",
                self.section3.number_of_points,
                number_of_read
            ));
        }
        // 第8節を読み込み
        read_section8(&mut reader)?;

        Ok(())
    }

    fn output_values(
        &self,
        writer: &mut FileWriter,
        level: u16,
        count: u32,
        longitude: &mut u32,
        latitude: &mut u32,
        boundary: &Boundary,
    ) -> anyhow::Result<()> {
        if 0 < level {
            for _ in 0..count {
                if boundary.contains(*longitude, *latitude) {
                    writeln!(
                        writer,
                        "{:.6},{:.6},{}",
                        (*longitude as f64) / 1_000_000f64,
                        (*latitude as f64) / 1_000_000f64,
                        self.section5.level_values[(level - 1) as usize],
                    )?;
                }
                *longitude += self.section3.longitude_increment;
                if self.section3.easternmost < *longitude {
                    *longitude = self.section3.westernmost;
                    *latitude -= self.section3.latitude_increment;
                }
            }
        } else {
            // レベル0は、欠測値であるため、出力しない
            (*longitude, *latitude) = move_lattice_for_missing_values(
                *longitude,
                *latitude,
                count,
                self.section3.longitude_increment,
                self.section3.latitude_increment,
                self.section3.westernmost,
                self.section3.easternmost,
            );
        }

        Ok(())
    }
}

/// 欠測値のときに、格子を移動する。
///
/// # 引数
///
/// * `longitude` - 現在の格子の経度。
/// * `latitude` - 現在の格子の緯度。
/// * `count` - 格子のレベル値が連続する数。
/// * `longitude_increment` - 経線方向の格子の移動量。
/// * `latitude_increment` - 緯線方向の格子の移動量。
/// * `lattice_width` - 経線方向の格子の幅。
/// * `westernmost` - 最西端の経度。
/// * `easternmost` - 最東端の経度。
///
/// # 戻り値
///
/// 移動後の格子の経度と緯度のタプル。
fn move_lattice_for_missing_values(
    longitude: u32,
    latitude: u32,
    count: u32,
    longitude_increment: u32,
    latitude_increment: u32,
    westernmost: u32,
    easternmost: u32,
) -> (u32, u32) {
    let mut longitude = longitude;
    let mut latitude = latitude;
    let lattice_width = easternmost - westernmost;
    // 格子を経線方向に移動する合計の度数
    let sum_of_lon_inc = longitude_increment as u64 * count as u64;
    // 格子を緯線方向に移動する格子数
    let lat_inc_times = sum_of_lon_inc / lattice_width as u64;
    // 緯線方向に格子を移動
    latitude -= latitude_increment * lat_inc_times as u32;
    // 経線方向に格子を移動
    // 格子が最東端に達したとき、次の格子は最西端かつ緯線南方向に1格子移動する。
    // このとき、経線方向に格子分移動しないため、緯線方向に移動する回数だけ、経線方向の移動を無効にする。
    // よって、`- (longitude_increment * lat_inc_times as u32)`している。
    longitude += ((sum_of_lon_inc % lattice_width as u64)
        - (longitude_increment as u64 * lat_inc_times)) as u32;
    if easternmost < longitude {
        // 上記と同様な理由で、`- longitude_increment`している。
        longitude = westernmost + (longitude - easternmost - longitude_increment);
        latitude -= latitude_increment;
    }

    (longitude, latitude)
}

/// ファイルから1バイト読み込み、u8型の値として返却する。
fn read_u8(reader: &mut FileReader) -> anyhow::Result<u8> {
    let mut buf = [0; 1];
    let size = reader.read(&mut buf)?;
    if size != 1 {
        return Err(anyhow!("failed to read a u8 value"));
    }

    Ok(u8::from_be_bytes(buf))
}

/// ファイルから2バイト読み込み、u16型の値として返却する。
fn read_u16(reader: &mut FileReader) -> anyhow::Result<u16> {
    let mut buf = [0; 2];
    let size = reader.read(&mut buf)?;
    if size != 2 {
        return Err(anyhow!("failed to read a u16 value"));
    }

    Ok(u16::from_be_bytes(buf))
}

/// ファイルから4バイト読み込み、u32型の値として返却する。
fn read_u32(reader: &mut FileReader) -> anyhow::Result<u32> {
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
fn read_section0(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section0_grib(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section0_document_domain(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a document domain"))?;
    match value {
        DOCUMENT_DOMAIN => Ok(()),
        _ => Err(anyhow!("a document domain is not {DOCUMENT_DOMAIN}")),
    }
}

/// 第0節 GRIB版番号を読み込んで、想定しているGRIB版番号であるか確認する。
fn read_section0_grib_version(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a grib version"))?;
    match value {
        GRIB_VERSION => Ok(()),
        _ => Err(anyhow!("a grib version is not {GRIB_VERSION}")),
    }
}

/// 第1節を読み込んで、確認する。
///
/// ファイルポインタが、第1節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第3節の開始位置に移動する。
/// なお、実装時点で、第2節は省略されている。
fn read_section1(reader: &mut FileReader) -> anyhow::Result<()> {
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
    read_section1_referenced_at(reader)?;
    // 作成ステータス
    read_section1_creation_status(reader)?;
    // 資料の種類
    read_section1_document_kind(reader)?;

    Ok(())
}

/// 第１節 GRIBマスター表バージョン番号を読み込んで、想定しているGRIBマスター表バージョン番号であるか確認する。
fn read_section1_grib_master_table_version(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section1_grib_local_table_version(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section1_referenced_at(reader: &mut FileReader) -> anyhow::Result<PrimitiveDateTime> {
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
fn read_section1_creation_status(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a creation status"))?;
    match value {
        CREATION_STATUS => Ok(()),
        _ => Err(anyhow!("a creation status is test product")),
    }
}

/// 第１節 資料の種類を読み込んで、想定している資料の種類であるか確認する。
fn read_section1_document_kind(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a document kind"))?;
    match value {
        DOCUMENT_KIND => Ok(()),
        _ => Err(anyhow!("a document kind is not {DOCUMENT_KIND}")),
    }
}

/// 第3節情報
struct Section3 {
    /// 資料点数
    pub number_of_points: u32,
    /// 最初（最も左上）の格子点の緯度（10^6度単位）
    pub northernmost: u32,
    /// 最初（最も左上）の格子点の経度（10^6度単位）
    pub westernmost: u32,
    /// 最後（最も右下）の格子点の緯度（10^6度単位）
    #[allow(dead_code)]
    pub southernmost: u32,
    /// 最後（最も右下）の格子点の経度（10^6度単位）
    pub easternmost: u32,
    /// i方向（経線方向）の増分（10^6度単位）
    pub longitude_increment: u32,
    /// j方向（緯線方向）の増分（10^6度単位）
    pub latitude_increment: u32,
}

/// 第3節を読み込んで、第3節の情報を返却する。
///
/// ファイルポインタが、第3節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第4節の開始位置に移動する。
fn read_section3(reader: &mut FileReader) -> anyhow::Result<Section3> {
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
        longitude_increment: horizontal_increment,
        latitude_increment: vertical_increment,
    })
}

/// 第3節 格子系定義の出典を読み込んで、想定している格子系定義の出典であるか確認する。
fn read_section3_grid_system_definition(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a grid system definition"))?;
    match value {
        GRID_SYSTEM_DEFINITION => Ok(()),
        _ => Err(anyhow!(
            "a grid system definition is not {GRID_SYSTEM_DEFINITION}"
        )),
    }
}

/// 第3節 資料点数を読み込んで、返却する。
fn read_section3_number_of_points(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a number of points in section 3"))
}

/// 第3節 格子系定義テンプレート番号を読み込んで、想定している格子系定義テンプレート番号であるか確認する。
fn read_section3_grid_system_definition_template(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section3_earth_figure(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a earth figure"))?;
    match value {
        EARTH_FIGURE => Ok(()),
        _ => Err(anyhow!("a earth figure is not {EARTH_FIGURE}")),
    }
}

/// 第3節 緯線に沿った格子点数を読み込んで、想定している点数であるか確認する。
fn read_section3_number_of_points_at_vertical(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section3_number_of_points_at_horizontal(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section3_creation_range_angle(reader: &mut FileReader) -> anyhow::Result<()> {
    let value = read_u32(reader).map_err(|_| anyhow!("failed to read a creation range angle"))?;
    match value {
        CREATION_RANGE_ANGLE => Ok(()),
        _ => Err(anyhow!(
            "a creation range angle is not {CREATION_RANGE_ANGLE}"
        )),
    }
}

/// 第3節 最初の格子点の緯度を読み込んで、返却する。
fn read_section3_northernmost_degree(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a northernmost grid point latitude"))
}

/// 第3節 最初の格子点の経度を読み込んで、返却する。
fn read_section3_westernmost_degree(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a westernmost grid point longitude"))
}

/// 第3節 最後の格子点の緯度を読み込んで、返却する。
fn read_section3_southernmost_degree(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a southernmost grid point latitude"))
}

/// 第3節 最後の格子点の経度を読み込んで、返却する。
fn read_section3_easternmost_degree(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a easternmost grid point longitude"))
}

/// 第3節 i方向（経線方向）の増分を読み込んで、想定している増分か確認する。
fn read_section3_horizontal_increment(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a horizontal increment"))
}

/// 第3節 j方向（緯線方向）の増分を読み込んで、想定している増分か確認する。
fn read_section3_vertical_increment(reader: &mut FileReader) -> anyhow::Result<u32> {
    read_u32(reader).map_err(|_| anyhow!("failed to read a vertical increment"))
}

/// 第3節 走査モードを読み込んで、想定しているモードか確認する。
fn read_section3_scanning_mode(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section4(reader: &mut FileReader) -> anyhow::Result<()> {
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
struct Section5 {
    /// 全資料点の数
    pub number_of_points: u32,
    /// 1データのビット数
    pub bits_per_data: u8,
    /// 今回の圧縮に用いたレベルの最大値
    pub max_level_at_file: u16,
    /// レベルの最大値
    #[allow(dead_code)]
    pub max_level: u16,
    /// レベルmに対応するデータ代表値
    /// レベル値と物理値(mm/h)の対応を格納するコレクション
    pub level_values: Vec<u16>,
}

/// 第5節を読み込んで、第3節の情報を返却する。
///
/// ファイルポインタが、第5節の開始位置にあることを想定している。
/// 関数終了後、ファイルポインタは第6節の開始位置に移動する。
fn read_section5(reader: &mut FileReader) -> anyhow::Result<Section5> {
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
fn read_section5_number_of_points(reader: &mut FileReader) -> anyhow::Result<u32> {
    // 第5節 節番号: 1byte
    read_u32(reader).map_err(|_| anyhow!("failed to read a number of points in section 5"))
}

/// 第5節 資料表現テンプレート番号を読み込み、想定している資料表現テンプレート番号であることを確認する。
fn read_section5_document_expression_template(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section5_bits_per_data(reader: &mut FileReader) -> anyhow::Result<u8> {
    let value = read_u8(reader).map_err(|_| anyhow!("failed to read a bits per data"))?;
    match value {
        BITS_PER_DATA => Ok(value),
        _ => Err(anyhow!("a bits per data is not {BITS_PER_DATA}")),
    }
}

/// 第5節 今回の圧縮に用いたレベルの最大値を読み込み、返却する。
fn read_section5_max_level_of_this_time(reader: &mut FileReader) -> anyhow::Result<u16> {
    read_u16(reader).map_err(|_| anyhow!("failed to read a max level of this time"))
}

/// 第5節 レベルの最大値を読み込み、返却する。
fn read_section5_max_level(reader: &mut FileReader) -> anyhow::Result<u16> {
    read_u16(reader).map_err(|_| anyhow!("failed to read a max level"))
}

/// 第5節 データ代表値の尺度因子を読み込み、想定している尺度因子であることを確認する。
fn read_section5_data_value_factor(reader: &mut FileReader) -> anyhow::Result<()> {
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
fn read_section6(reader: &mut FileReader) -> anyhow::Result<()> {
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

/// 第8節を読み込んで、確認する。
fn read_section8(reader: &mut FileReader) -> anyhow::Result<()> {
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

/// 1セットのランレングス符号化（圧縮）を展開する。
///
/// valuesの最初の要素はレベル値で、それ以降はランレングス値である。
/// これを1セットのランレングス符号化とする。
/// ランレングス値を含まない場合のvaluesの要素数は1で、ランレングス値を含む場合のvaluesの要素数は2以上である。
///
/// この関数が展開する、GRIB2資料テンプレート7.200（気象庁定義資料テンプレート）で利用されているランレングス符号化を以下に示す。
///
/// * 格子点値が取りうるレベル値
///   * レベル値は2次元矩形領域の格子点上に存在し、0以上MAXV以下の整数を取る。
///   * ここでMAXVは、GRIB資料表現テンプレート5.200（気象庁定義資料表現テンプレート）第5節13-14オクテットで示される「今回の圧縮に用いたレベルの最大値」である。
///     * 第5節15-16オクテットの「レベルの最大値」ではないことに注意すること。
/// * 2次元データの1次元化
///   * 主走査方向を2次元矩形領域の左から右（通常西から東）、副走査方向を上から下（通常北から南）として、2次元データを1次元化する。
///     * データは最も左上の格子点の値から始まり、東方向に向かって格子点のレベル値を記録する。
///     * その緯度の最東端に達したら、下の最西端の格子点に移動して、上記同様に格子点のレベル値を記録する。
///   * 最初のデータは最も左上の格子点の値であり、最後のデータは最も右下の格子点の値である。
/// * ランレングス符号化後の1格子点値当りのビット数（NBIT）
///   * NBITは、ランレングス符号化されたデータ列の中で、レベル値及びランレングス値を表現するビット数である。
///   * NBITは、GRIB2資料表現テンプレート5.200第5節12オクテットで示される「1データのビット数」である。
/// * 1セット内のレベル値とランレングス値の配置
///   * ランレングス符号化されたデータ列の中で0以上MAXV以下の値は各格子点のレベル値で、MAXVよりも大きな値はランレングス値である。
///   * 1セットは、最初にレベル値を配置し、もしその値が連続するのであれば後ろにランレングス値を付加して作成される。
///   * MAXVよりも大きな値が続く場合、それらすべては当該セットのランレングス値である。
///   * データに、MAXV以下の値が現れた時点で当該セットが終了し、このMAXV以下の値は次のセットのレベル値となる。
///   * なお、同じレベル値が連続しない場合はランレングスは付加されず、次のセットに移る。
/// * ランレングス符号化方法
///   * (2 ^ NBIT - MAXV)よりも大きなランレングスが必要となった場合、1データでは表現することができない。
///   * これに対応するために、2つ以上のランレングス値を連続させてランレングスを表現するが、連続したデータの単純な総和をランレングスとしても圧縮効率があがらない。
///   * よって、LNGU(=2 ^ NBIT - 1 - MAXV)進数を用いてランレングスを表現する。
///   * レベル値のすぐ後に続く最初のランレングス値(data1)をLNGU進数の1桁目 RL1={LNGU ^ (1 - 1) * (data1 - (MAXV + 1))}とする。
///   * それ以降n番目のランレングス値(dataN)は LNGU進数のn桁目 RLn={LNGU ^ (n - 1) * (dataN - (MAXV + 1))}とする。
///   * 最終的なランレングスは、それらの「総和 + 1(RL = ΣRLi + 1)」となる。
/// * ランレングス符号化例
///   * NBIT = 4、MAXV = 10とした場合、LNGU = 2 ^ 4 - 1 - 10 = 16 - 1 - 10 = 5となる。
///   * ランレングス符号化列 = {3, 9, 12, 6, 4, 15, 2, 1, 0, 13, 12, 2, 3}は、以下の通り展開される。
///   * {3, 9, 9, 6, 4, 4, 4, 4, 4, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3}
///   * レベル値とランレングス値のセット{9, 12}
///     * 9がレベル値で12がランレングス値である。
///     * 12の次は6であり、10以下であるため6はレベル値である。
///     * RL1 = 5 ^ (1 - 1) * (12 - (10 + 1)) = 1 * 1 = 1
///     * RL = 1 + 1 = 2
///     * よって、9が２つ連続する。
///   * レベル値とランレングス値のセット{0, 13, 12}
///     * 0がレベル値で13と12がランレングス値である。
///     * RL1 = 5 ^ (1 - 1) * (13 - (10 + 1)) = 1 * 2 = 2
///     * RL2 = 5 ^ (2 - 1) * (12 - (10 + 1)) = 5 * 1 = 5
///     * RL = 2 + 5 + 1 = 8
///     * よって、0が8連続する。
///
/// # 引数
///
/// * `values` - 1セットのランレングス圧縮データ。
/// * `maxv` - 今回の圧縮に用いたレベルの最大値（第5節 13-14オクテット）。
/// * `lngu` - レベル値またはランレングス値のビット数をnbitとしたときの、2 ^ nbit -1 - maxvの値。
///
/// # 戻り値
///
/// レベル値とそのレベル値を繰り返す数を格納したタプル。
fn expand_run_length(values: &[u16], maxv: u16, lngu: u16) -> (u16, u32) {
    assert!(values[0] <= maxv, "values[0]={}, maxv={}", values[0], maxv);

    // ランレングス圧縮されていない場合
    if values.len() == 1 {
        return (values[0], 1);
    }

    // ランレングス圧縮を展開
    let values: Vec<u32> = values.iter().map(|v| *v as u32).collect();
    let lngu = lngu as u32;
    let maxv = maxv as u32;
    let count: u32 = values[1..]
        .iter()
        .enumerate()
        .map(|(i, &v)| lngu.pow(i as u32) * (v - (maxv + 1)))
        .sum();

    (values[0] as u16, count + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_FILE: &'static str = "fixtures/sample.bin";
    const SAMPLE_MAX_LEVEL_THIS_TIME: u16 = 77;

    #[test]
    fn can_read_grib_file() {
        let mut reader = BufReader::new(File::open(SAMPLE_FILE).unwrap());
        // 第0節を読み込み
        assert!(read_section0(&mut reader).is_ok());

        // 第1節を読み込み
        assert!(read_section1(&mut reader).is_ok());

        // 第3節を読み込み
        let section3 = read_section3(&mut reader).unwrap();
        assert_eq!(section3.number_of_points, 2560 * 3360);
        assert_eq!(section3.northernmost, 47995833);
        assert_eq!(section3.westernmost, 118006250);
        assert_eq!(section3.southernmost, 20004167);
        assert_eq!(section3.easternmost, 149993750);
        assert_eq!(section3.longitude_increment, 12500);
        assert_eq!(section3.latitude_increment, 8333);

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
        // 節の長さ: 4bytes
        let length = read_u32(&mut reader).unwrap();
        // 節番号
        let section_number = read_u8(&mut reader).unwrap();
        assert_eq!(section_number, 7);
        // ランレングス圧縮オクテット列をスキップ
        reader.seek_relative((length - (4 + 1)) as i64).unwrap();

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

    #[test]
    fn expand_run_length0() {
        let nbit = 4;
        let maxv = 10;
        let lngu = 2u16.pow(nbit) - 1 - maxv;
        let values = vec![3u16];
        let expected = (3u16, 1u32);
        assert_eq!(expected, expand_run_length(&values, maxv, lngu));
    }

    #[test]
    fn expand_run_length1() {
        let nbit = 4;
        let maxv = 10;
        let lngu = 2u16.pow(nbit) - 1 - maxv;
        let values = vec![9u16, 12];
        let expected = (9u16, 2u32);
        assert_eq!(expected, expand_run_length(&values, maxv, lngu));
    }

    #[test]
    fn expand_run_length2() {
        let nbit = 4;
        let maxv = 10;
        let lngu = 2u16.pow(nbit) - 1 - maxv;
        let values = vec![4u16, 15];
        let expected = (4u16, 5u32);
        assert_eq!(expected, expand_run_length(&values, maxv, lngu));
    }

    #[test]
    fn expand_run_length3() {
        let nbit = 4;
        let maxv = 10;
        let lngu = 2u16.pow(nbit) - 1 - maxv;
        let values = vec![0u16, 13, 12];
        let expected = (0u16, 8u32);
        assert_eq!(expected, expand_run_length(&values, maxv, lngu));
    }

    #[test]
    fn should_be_contained_by_boundary() {
        let boundary = Boundary {
            northernmost: Some(36000000),
            southernmost: Some(35000000),
            westernmost: Some(135000000),
            easternmost: Some(136000000),
        };
        let coordinates = vec![
            (135000000, 36000000),
            (136000000, 36000000),
            (135000000, 35000000),
            (136000000, 35000000),
            (135500000, 35500000),
        ];
        for dataset in coordinates {
            assert!(boundary.contains(dataset.0, dataset.1), "{:?}", dataset);
        }
    }

    #[test]
    fn should_be_not_contained_by_boundary() {
        let boundary = Boundary {
            northernmost: Some(36000000),
            southernmost: Some(35000000),
            westernmost: Some(135000000),
            easternmost: Some(136000000),
        };
        let coordinates = vec![
            (134900000, 36000000),
            (135000000, 36100000),
            (136100000, 36000000),
            (135000000, 34900000),
        ];
        for dataset in coordinates {
            assert!(!boundary.contains(dataset.0, dataset.1), "{:?}", dataset);
        }
    }

    #[test]
    fn move_lattice_for_missing_value1() {
        // 現在の緯度と経度が135度、40度で、レベル0が10個連続したとする。
        // 経線方向の増加量1度、緯線方向の増加量1度
        // 最西端130度、最東端150度
        // 移動後の格子の座標は145度、40度
        let expected = (145000000u32, 40000000u32);
        let lattice = move_lattice_for_missing_values(
            135000000u32,
            40000000u32,
            10,
            1000000,
            1000000,
            130000000,
            150000000,
        );
        assert_eq!(lattice, expected);
    }

    #[test]
    fn move_lattice_for_missing_value2() {
        // 現在の緯度と経度が140度、40度で、レベル0が10個連続したとする。
        // 経線方向の増加量1度、緯線方向の増加量1度
        // 最西端130度、最東端150度
        // 移動後の格子の座標は150度、40度
        let expected = (150000000u32, 40000000u32);
        let lattice = move_lattice_for_missing_values(
            140000000u32,
            40000000u32,
            10u32,
            1000000u32,
            1000000u32,
            130000000u32,
            150000000u32,
        );
        assert_eq!(lattice, expected);
    }

    #[test]
    fn move_lattice_for_missing_value3() {
        // 現在の緯度と経度が140度、40度で、レベル0が11個連続したとする。
        // 経線方向の増加量1度、緯線方向の増加量1度
        // 最西端130度、最東端150度
        // 移動後の格子の座標は130度、39度
        let expected = (130000000u32, 39000000u32);
        let lattice = move_lattice_for_missing_values(
            140000000u32,
            40000000u32,
            11u32,
            1000000u32,
            1000000u32,
            130000000u32,
            150000000u32,
        );
        assert_eq!(lattice, expected);
    }

    #[test]
    fn move_lattice_for_missing_value4() {
        // 現在の緯度と経度が145度、40度で、レベル0が50個連続したとする。
        // 経線方向の増加量1度、緯線方向の増加量1度
        // 最西端130度、最東端150度
        // 移動後の格子の座標は134度、37度
        let expected = (132000000u32, 37000000u32);
        let lattice = move_lattice_for_missing_values(
            145000000u32,
            40000000u32,
            50u32,
            1000000u32,
            1000000u32,
            130000000u32,
            150000000u32,
        );
        assert_eq!(lattice, expected);
    }
}

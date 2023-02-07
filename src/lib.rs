use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::str;

use anyhow::anyhow;
use time::{Date, Month, PrimitiveDateTime, Time};

/// 資料分野: 気象分野
const DOCUMENT_DOMAIN: u8 = 0;
/// GRIB版番号
const GRIB_VERSION: u8 = 2;
/// GRIBマスター表バーション番号
const GRIB_MASTER_TABLE_VERSION: u8 = 2;
/// GRIB地域表バージョン番号
const GRIB_LOCAL_TABLE_VERSION: u8 = 1;
/// 作成ステータス: 現業プロダクト
const CREATION_STATUS: u8 = 0;
/// 資料の種類: 解析プロダクト
const DOCUMENT_KIND: u8 = 0;

pub struct GRIB2Info {
    /// grib2は世界標準時で日時を記録
    pub date_time: PrimitiveDateTime,
    /// 資料点数
    pub number_of_points: u32,
    /// 最初の格子点の緯度を10^6倍した値
    pub top: u32,
    /// 最初の格子点の経度を10^6倍した値
    pub left: u32,
    /// 最後の格子点の緯度を10^6倍した値
    pub bottom: u32,
    /// 最後の格子点の経度を10^6倍した値
    pub right: u32,
    /// 経度方向の座標の増分値を10^6倍した値
    pub h_increment: u32,
    /// 緯度方向の座標の増分値を10^6倍した値
    pub v_increment: u32,
    /// 1データ（レベル値とランレングス値）のビット数
    pub data_per_bits: u8,
    /// 今回の圧縮に用いたレベルの最大値
    pub maxv: u16,
    /// レベル値と物理値(mm/h)の対応を格納するコレクション
    pub level_values: HashMap<u16, u16>,
}

impl GRIB2Info {}

/// ファイルの先頭に"GRIB"が記録されているか確認する。
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

/// 資料分野を読み込んで、想定している資料分野であるか確認する。
///
/// ファイル・ポインタが、第0節 GRIBの直後にある必要がある。
fn read_document_domain(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第0節 保留: 2bytes
    reader.seek_relative(2)?;
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a document domain"));
    }
    match u8::from_be_bytes(buf) {
        DOCUMENT_DOMAIN => Ok(()),
        _ => Err(anyhow!("adocument domain is not {DOCUMENT_DOMAIN}")),
    }
}

/// GRIB版番号を読み込んで、想定しているGRIB版番号であるか確認する。
///
/// ファイル・ポインタが、第0節 資料分野の直後にある必要がある。
fn read_grib_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a grib version"));
    }
    match u8::from_be_bytes(buf) {
        GRIB_VERSION => Ok(()),
        _ => Err(anyhow!("a grib version is not {GRIB_VERSION}")),
    }
}

/// GRIBマスター表バージョン番号を読み込んで、想定しているGRIBマスター表バージョン番号であるか確認する。
///
/// ファイル・ポインタが、第0節 GRIB版番号の直後にある必要がある。
fn read_grib_master_table_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    // 第0節 GRIB全体の長さ: 8bytes
    // 第1節 節の長さ: 4bytes
    // 第1節 節番号: 1bytes
    // 第1節 作成中枢の識別: 2bytes
    // 第1節 作成副中枢: 2bytes
    reader.seek_relative(17)?;
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a grib master table version"));
    }
    match u8::from_be_bytes(buf) {
        GRIB_MASTER_TABLE_VERSION => Ok(()),
        _ => Err(anyhow!(
            "a grib master table version is not {GRIB_MASTER_TABLE_VERSION}"
        )),
    }
}

/// GRIB地域差バージョン番号を読み込んで、想定しているGRIB地域差バージョン番号であるか確認する。
///
/// ファイル・ポインタが、第1節 GRIBマスター表バージョン番号の直後にある必要がある。
fn read_grib_local_table_version(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a grib local table version"));
    }
    match u8::from_be_bytes(buf) {
        GRIB_LOCAL_TABLE_VERSION => Ok(()),
        _ => Err(anyhow!(
            "a grib local table version is not {GRIB_LOCAL_TABLE_VERSION}"
        )),
    }
}

/// 資料の参照日時を読み込んで返却する。
///
/// ファイル・ポインタが、第1節 GRIB地域表バージョン番号の直後にある必要がある。
fn read_reference_date_time(reader: &mut BufReader<File>) -> anyhow::Result<PrimitiveDateTime> {
    // 第1節 参照時刻の意味: 1byte
    reader.seek_relative(1)?;
    // 資料の参照時刻（年）
    let mut buf = [0; 2];
    let length = reader.read(&mut buf)?;
    if length != 2 {
        return Err(anyhow!("failed to read a reference year"));
    }
    let year = u16::from_be_bytes(buf);
    // 資料の参照時刻（月以降）
    let mut parts = Vec::new();
    for _ in 0..5 {
        let mut buf = [0; 1];
        let length = reader.read(&mut buf)?;
        if length != 1 {
            return Err(anyhow!("failed to read for any reference time parts"));
        }
        parts.push(u8::from_be_bytes(buf));
    }
    // 日付と時刻を構築
    let month = Month::try_from(parts[0])?;
    let date = Date::from_calendar_date(year as i32, month, parts[1])?;
    let time = Time::from_hms(parts[2], parts[3], parts[4])?;

    Ok(PrimitiveDateTime::new(date, time))
}

/// 作成ステータスを読み込んで、想定している作成ステータスであるか確認する。
///
/// ファイル・ポインタが、第1節 資料の参照時刻（秒）の直後にある必要がある。
fn read_creation_status(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a creation status"));
    }
    match u8::from_be_bytes(buf) {
        CREATION_STATUS => Ok(()),
        _ => Err(anyhow!("creation status is not {CREATION_STATUS}")),
    }
}

/// 資料の種類を読み込んで、想定している資料の種類であるか確認する。
///
/// ファイルポインタが、第1節 作成ステータスの直後にある必要がある。
fn read_document_kind(reader: &mut BufReader<File>) -> anyhow::Result<()> {
    let mut buf = [0; 1];
    let length = reader.read(&mut buf)?;
    if length != 1 {
        return Err(anyhow!("failed to read a document kind"));
    }
    match u8::from_be_bytes(buf) {
        DOCUMENT_KIND => Ok(()),
        _ => Err(anyhow!("document kind is not {DOCUMENT_KIND}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    const SAMPLE_FILE: &'static str = "fixtures/20200707000000_grib2.bin";

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
    }
}

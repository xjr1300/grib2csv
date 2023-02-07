use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::str;

use anyhow::anyhow;
use time::OffsetDateTime;

pub struct GRIB2Info {
    /// grib2は世界標準時で日時を記録しているが、日本標準時で記録
    pub date_time: OffsetDateTime,
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

impl GRIB2Info {
    fn dummy_default() -> Self {
        Self {
            date_time: OffsetDateTime::now_utc(),
            number_of_points: 0,
            top: 47995833,
            left: 118006250,
            bottom: 20004167,
            right: 149993750,
            h_increment: 12500,
            v_increment: 8333,
            data_per_bits: 8,
            maxv: 98,
            level_values: HashMap::new(),
        }
    }
}

/// grib2ファイルの先頭から4倍と読み込んで、"GRIB"が記録されているか確認する。
fn read_grib<R: Read>(reader: &mut BufReader<R>) -> anyhow::Result<()> {
    let mut buf = [0; 4];

    let length = reader.read(&mut buf)?;
    if length != 4 {
        return Err(anyhow!("can not read `GRIB` from the file"));
    }
    let s = str::from_utf8(buf.as_slice())?;
    match s {
        "GRIB" => Ok(()),
        _ => Err(anyhow!("can not read `GRIB` from the file")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    const SAMPLE_FILE: &'static str = "fixtures/20200707000000_grib2.bin";

    /// grib2ファイルを開いて、GRIBを読み込めるか確認する。
    #[test]
    fn can_read_grib() {
        let mut reader = BufReader::new(File::open(SAMPLE_FILE).unwrap());
        assert!(read_grib(&mut reader).is_ok());
    }

    /// grib2ファイル以外から、GRIBを読み込めないことを確認する。
    #[test]
    fn can_not_read_grib() {
        let dummy = "XXXX";
        let mut reader = BufReader::new(dummy.as_bytes());
        assert!(read_grib(&mut reader).is_err());
    }
}

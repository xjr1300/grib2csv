use std::collections::HashMap;
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

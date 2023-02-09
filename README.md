# grib2csv

GRIB2通報式による1kmメッシュ解析雨量または降水短時間予報データを、CSV形式のファイルに変換します。

## 使用方法

```bash
grib2csv [OPTIONS] <INPUT> <OUTPUT>
```

## 実行例

```bash
grib2csv -n 36000000 -s 35000000 -w 135000000 -e 136000000 input.bin output.csv
```

## 引数

* `<INPUT>` - 変換するGRIB2ファイルのパス。
* `<OUTPUT>` - 変換した結果を出力するCSV形式のパス。

## オプション

* `-n`, `--northernmost` `<NORTHERNMOST>` - CSVファイルに出力する格子点の最北端の緯度(例: 36532213)
* `-s`, `--southernmost` `<SOUTHERNMOST>` - CSVファイルに出力する格子点の最南端の緯度(例: 35432213)
* `-w`, `--westernmost` `<WESTERNMOST>` - CSVファイルに出力する格子点の最西端の経度(例: 135532213)
* `-e`, `--easternmost` `<EASTERNMOST>` - CSVファイルに出力する格子点の最東端の経度(例: 136532213)
* `-h`, `--help` - ヘルプを出力

## 出力結果

出力したCSVファイルには、経度、緯度及び物理値(mm/h)が、この順番でカンマ(`,`)区切りで出力されています。
また、CSVファイルの最初の行はヘッダです。

## 注意事項

`-n`オプションなど出力する格子点の範囲を指定する場合、指定したい度単位の緯度や経度を1,000,000した整数値で指定してください。

## テスト

```bash
# 単体テスト
cargo test
# 統合テスト
cargo test -- --ignored
```

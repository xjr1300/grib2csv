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

## Windowsにおけるコンパイル

> `Ubuntu`など`Linux`系または`macOS`などで、`grib2csv`を実行することをおすすめします。
> `Windows`より`grib2csv`のコンパイルがとても簡単です。
> `Ubuntu`など`Linux`系または`macOS`などで、`grib2csv`をコンパイルする方法は、`Windows`の説明で理解できると思います。

### Microsoft C++ Build Toolのインストール

1. `Microsoft C++ Build Tool`インストーラーを[ここから](https://visualstudio.microsoft.com/ja/visual-cpp-build-tools/)ダウンロードします。
2. `Microsoft C++ Build Tool`インストーラーを実行します。
3. `Microsoft C++ Build Tool`インストーラー画面で、以下をチェックします。
   * `ワークロード` > `C++によるデスクトップ開発`
   * `言語パック` > `英語`
   * `言語パック` > `日本語`
4. `Microsoft C++ Build Tool`インストーラー画面で、`インストール`ボタンをクリックします。
5. `Windows`の再起動が促されたら、再起動します。

## Gitのインストール

1. `Git`のインストーラーを[ここから](https://github.com/git-for-windows/git/releases/download/v2.39.1.windows.1/Git-2.39.1-64-bit.exe)ダウンロードします。
2. `Git`のインストーラーを実行します。
3. [ここ](https://www.curict.com/item/60/60bfe0e.html)を参考に、`Git`をインストールします。
    * `Select Components`では、デフォルトで`Next`ボタンをクリックします。
    * `Choosing the default editor used by Git`では、`Use Vim ...`を選択して、`Next`ボタンをクリックします。
    * `Adjusting the name of the initial branch in new repositories`では、`Override the default branch name for new repositories`をチェックして、テキストに`main`を入力して、`Next`ボタンをクリックします。

## Rustのインストール

1. `Rust`のインストーラーを[ここから](https://static.rust-lang.org/dist/rust-1.67.0-x86_64-pc-windows-msvc.msi)ダウンロードします。
2. `Rust`のインストーラーを実行します。
3. `コマンド・プロンプト`または`PowerShell`を起動して、以下のコマンドを打ちます。
   * バージョン番号が表示されたら、正常にRustをインストールできています。

```bash
rustc --version
```

## grib2csvのコンパイルと実行

### grib2csvのソースコードのダウンロード

1. `PowerShell`を実行します。
2. `cd`コマンドで、`grib2csv`のソースコードをダウンロード（クローン）するディレクトリをカレントにします。
3. 以下のコマンドを入力して、`grib2csv`のソースコードをクローンします。

```bash
git clone https://github.com/xjr1300/grib2csv.git
```

### grib2csvのコンパイル

1. `grib2csv`のソースコードをクローンした後、`grib2csv`ディレクトリに移動して、コンパイルします。
    * `Rust`を新規にインストールした場合、クレート情報を更新するため、少し時間がかかります。
    * `grib2csv`実行形式ファイルは、`./target/release/grib2csv.exe`に出力されます。

```bash
cd grib2csv
cargo build --release
```

### grib2csvの実行

1. パスに`./target/release/`追加するか、`grib2csv.exe`をパスが通っているディレクトリにコピー／移動します。
2. `grib2csv.exe`を実行します。

# grib2csv

GRIB2通報式による1kmメッシュ解析雨量または降水短時間予報データを、CSV形式のファイルに変換します。

## 1. 使用方法

```bash
grib2csv [OPTIONS] <INPUT> <OUTPUT>
```

### 1.1. 必須引数

* `<INPUT>` - 変換するGRIB2ファイルのパス。
* `<OUTPUT>` - 変換した結果を出力するCSVファイルのパス。

### 1.2. オプション引数

* `-n`, `--northernmost` `<NORTHERNMOST>` - CSVファイルに出力する格子点の最北端の緯度(例: 36532213)
* `-s`, `--southernmost` `<SOUTHERNMOST>` - CSVファイルに出力する格子点の最南端の緯度(例: 35432213)
* `-w`, `--westernmost` `<WESTERNMOST>` - CSVファイルに出力する格子点の最西端の経度(例: 135532213)
* `-e`, `--easternmost` `<EASTERNMOST>` - CSVファイルに出力する格子点の最東端の経度(例: 136532213)
* `--no-header` - CSVファイルにヘッダを出力しない
* `-h`, `--help` - ヘルプを出力
* `-v`, `--version` - バージョンを出力

### 1.3. 注意事項

`-n`オプションなど出力する格子点の範囲を指定する場合、指定したい度単位の緯度や経度を1,000,000倍したときの整数部を指定してください。

### 1.4. 使用例

```bash
# 入力ファイルに記録されているすべての格子点を、CSVファイルに出力
grib2csv input.bin output.csv
# 入力ファイルに記録されている格子点の内、緯度35度から36度かつ経度135度から136度に含まれる格子点を、
# CSVファイルに出力
grib2csv -n 36000000 -s 35000000 -w 135000000 -e 136000000 input.bin output.csv
# 入力ファイルに記録されている格子点の内、緯度35度以上かつ経度135度以上の格子点を、CSVファイルに出力
grib2csv -s 35000000 -w 135000000 input.bin output.csv
```

## 2. 実行結果

`grib2csv`が出力したCSVファイルには、経度、緯度及び物理値(mm/h)が、この順番でカンマ(`,`)区切りで記録されています。

## 3. Windowsでソースコードをコンパイルする

> `Ubuntu`など`Linux`系または`macOS`などで、`grib2csv`をコンパイルする方法は、`Windows`の説明で理解できると思います。

### 3.1. Microsoft C++ Build Toolのインストール

1. `Microsoft C++ Build Tool`インストーラーを[ここから](https://visualstudio.microsoft.com/ja/visual-cpp-build-tools/)ダウンロードします。
2. `Microsoft C++ Build Tool`インストーラーを実行します。
3. `Microsoft C++ Build Tool`インストーラー画面で、以下をチェックします。
   * `ワークロード` > `C++によるデスクトップ開発`
   * `言語パック` > `英語`
   * `言語パック` > `日本語`
4. `Microsoft C++ Build Tool`インストーラー画面で、`インストール`ボタンをクリックします。
5. `Windows`の再起動が促されたら、再起動します。

### 3.2. Gitのインストール

1. `Git`のインストーラーを[ここから](https://github.com/git-for-windows/git/releases/download/v2.39.1.windows.1/Git-2.39.1-64-bit.exe)ダウンロードします。
2. `Git`のインストーラーを実行します。
3. [ここ](https://www.curict.com/item/60/60bfe0e.html)を参考に、`Git`をインストールします。
    * `Select Components`では、デフォルトで`Next`ボタンをクリックします。
    * `Choosing the default editor used by Git`では、`Use Vim ...`を選択して、`Next`ボタンをクリックします。
    * `Adjusting the name of the initial branch in new repositories`では、`Override the default branch name for new repositories`をチェックして、テキストに`main`を入力して、`Next`ボタンをクリックします。

### 3.3. Rustのインストール

1. `Rust`のインストーラーを[ここから](https://static.rust-lang.org/dist/rust-1.67.0-x86_64-pc-windows-msvc.msi)ダウンロードします。
2. `Rust`のインストーラーを実行します。
3. `コマンド・プロンプト`または`PowerShell`を起動して、以下のコマンドを打ちます。
   * バージョン番号が表示されたら、正常にRustをインストールできています。

```bash
rustc --version
```

### 3.4. grib2csvのコンパイルと実行

#### 3.4.1. grib2csvのソースコードのダウンロード

1. `PowerShell`を実行します。
2. `cd`コマンドで、`grib2csv`のソースコードをダウンロード（クローン）するディレクトリをカレントにします。
3. 以下のコマンドを入力して、`grib2csv`のソースコードをクローンします。

```bash
git clone https://github.com/xjr1300/grib2csv.git
```

#### 3.4.2. grib2csvのコンパイル

1. `grib2csv`のソースコードをクローンした後、`grib2csv`ディレクトリに移動して、コンパイルします。
    * `Rust`を新規にインストールした場合、クレート情報を更新するため、少し時間がかかります。
    * `grib2csv`実行形式ファイルは、`./target/release/grib2csv.exe`に出力されます。

```bash
cd grib2csv
cargo build --release
```

#### 3.4.3. grib2csvの実行

1. パスに`./target/release/`追加するか、`grib2csv.exe`をパスが通っているディレクトリにコピー／移動します。
2. `grib2csv.exe`を実行します。

## 4. テスト

```bash
# 単体テスト
cargo test
# 統合テスト
cargo test -- --ignored
```

## 5. 改訂履歴

* 0.0.1
  * 2023-02-09 リリース
* 0.0.2
  * 2023-02-10 リリース
  * ヘルプやREADMEを修正
  * `--no-header`引数を追加

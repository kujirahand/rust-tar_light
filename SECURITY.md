# Security Report

This document describes security risks and mitigations.

このドキュメントは、セキュリティリスクと対策を記載しています。

## 発見された脆弱性

### 1. パストラバーサル攻撃 (CWE-22) - 🔴 重大

**リスクレベル**: Critical

**説明**: `unpack()` 関数は、TARアーカイブ内のファイル名を検証せずにそのまま使用しているため、`../` を含むパスにより、意図したディレクトリ外にファイルを書き込むことが可能です。

**影響**:
- システムファイルの上書き
- 任意の場所へのファイル書き込み
- 権限昇格の可能性

**概念実証**:
```rust
// 悪意のあるTARアーカイブを作成
let header = TarHeader::new("../../../etc/passwd".to_string(), 0o644, data.len());
```

**対策**:
```rust
// unpack関数内でパスをサニタイゼーション
fn sanitize_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    
    // 絶対パスを拒否
    if path.is_absolute() {
        return None;
    }
    
    // パス成分を正規化し、'..' を検出
    let mut safe_path = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => safe_path.push(name),
            std::path::Component::ParentDir => return None, // '..' を拒否
            _ => return None,
        }
    }
    
    Some(safe_path)
}
```

**テスト**: `security_test_unpack_path_traversal`, `security_test_unpack_absolute_path`

---

### 2. シンボリックリンク攻撃 (CWE-59) - 🟡 中程度

**リスクレベル**: Medium

**説明**: TARアーカイブ内のシンボリックリンクが処理されると、アーカイブ外のファイルシステムへのアクセスが可能になる可能性があります。

**現在の対策**: `read_tar()` 関数は通常ファイル以外をフィルタリングしていますが、`pack()` 関数がシンボリックリンクをどう扱うかは不明瞭です。

**推奨対策**:
- シンボリックリンクの明示的な処理ポリシーを決定
- シンボリックリンクを追跡しない、またはアーカイブ内でのみ有効なリンクに限定

**テスト**: `security_test_symlink_in_archive`, `security_test_pack_symlink_handling`

---

### 3. 整数オーバーフロー (CWE-190) - 🟡 中程度

**リスクレベル**: Medium

**説明**: TARヘッダーの `size` フィールドに `u64::MAX` などの巨大な値が含まれている場合、メモリ割り当てやバッファ計算で問題が発生する可能性があります。

**影響**:
- DoS攻撃 (メモリ枯渇)
- バッファオーバーフロー
- パニックやクラッシュ

**現在の対策**: `read_tar()` 関数は利用可能なデータサイズを超えた読み取りを試みないため、部分的に保護されています。

**推奨対策**:
```rust
const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB制限

if header.size > MAX_FILE_SIZE {
    eprintln!("File size too large: {}", header.size);
    continue;
}
```

**テスト**: `security_test_integer_overflow`, `security_test_size_mismatch`

---

### 4. Zipスリップ脆弱性 (CWE-23) - 🔴 重大

**リスクレベル**: Critical

**説明**: パストラバーサル攻撃の一種で、圧縮されたアーカイブ（.tar.gz）を展開する際に、絶対パスや相対パスを使用してファイルシステムの任意の場所にファイルを書き込むことができます。

**影響**: パストラバーサル攻撃と同じ

**推奨対策**: パストラバーサル攻撃の対策と同じ

**テスト**: `security_test_unpack_path_traversal`, `security_test_unpack_absolute_path`

---

### 5. ファイル上書き (CWE-73) - 🟢 低

**リスクレベル**: Low

**説明**: `unpack()` 関数は既存のファイルを警告なしに上書きします。

**影響**:
- データ損失
- 意図しないファイル置換

**推奨対策**:
- 上書き前に確認を求めるオプション
- 上書き禁止モード
- バックアップ作成オプション

**テスト**: `security_test_unpack_overwrites_existing`

---

### 6. 特殊文字インジェクション (CWE-75) - 🟡 中程度

**リスクレベル**: Medium

**説明**: ファイル名にNULLバイト、改行、特殊文字が含まれている場合、ファイルシステム操作やログに予期しない影響を与える可能性があります。

**影響**:
- ファイル作成エラー
- ログインジェクション
- パス解析の混乱

**現在の対策**: `read_tar_str()` 関数はNULLバイトで文字列を終端処理しています。

**推奨対策**:
```rust
fn is_safe_filename(name: &str) -> bool {
    !name.contains('\0') && 
    !name.contains('\n') && 
    !name.contains('\r') &&
    !name.is_empty()
}
```

**テスト**: `security_test_null_byte_injection`, `security_test_special_characters`

---

### 7. デバイスファイル攻撃 (CWE-367) - 🟢 低

**リスクレベル**: Low

**説明**: TARアーカイブにデバイスファイル、FIFO、ディレクトリエントリが含まれている場合、特権昇格やDoS攻撃に悪用される可能性があります。

**現在の対策**: `read_tar()` 関数は通常ファイル (typeflag '0' または 0) のみを処理し、他のタイプをフィルタリングしています。

**テスト**: `security_test_device_file_in_archive`

---

### 8. チェックサムバイパス (CWE-354) - 🟢 低

**リスクレベル**: Low

**説明**: チェックサム検証が自動的に行われないため、破損したまたは改ざんされたアーカイブを検出できません。

**影響**:
- データ整合性の欠如
- 意図しないデータ破損の見逃し

**推奨対策**:
- デフォルトでチェックサム検証を有効化
- 検証失敗時の明示的なエラー処理

**テスト**: `security_test_invalid_checksum`

---

### 9. フィールドオーバーフロー保護 - ✅ 対策済み

**説明**: 過度に長いフィールド値（名前、プレフィックス、ユーザー名など）が適切にトリミングされています。

**対策**: `create_tar_header()` 関数は各フィールドの最大長を強制しています。

**テスト**: `security_test_oversized_name`, `security_test_oversized_prefix`, `security_test_all_fields_oversized`

---

### 10. 深いネストパス (CWE-400) - 🟢 低

**リスクレベル**: Low

**説明**: 極端に深いディレクトリ構造がリソース枯渇を引き起こす可能性があります。

**影響**:
- ディスク容量の浪費
- inode枯渇
- パス長制限の問題

**推奨対策**:
```rust
const MAX_PATH_DEPTH: usize = 100;

if path.components().count() > MAX_PATH_DEPTH {
    eprintln!("Path too deep: {}", path);
    continue;
}
```

**テスト**: `security_test_deeply_nested_path`

---

## 推奨される緩和策の優先順位

### 高優先度 (即時対応推奨)

1. **パストラバーサル対策の実装** - `unpack()` 関数にパスサニタイゼーションを追加
2. **最大ファイルサイズ制限** - DoS攻撃防止のため

### 中優先度

3. **シンボリックリンクポリシーの明確化** - ドキュメントと実装
4. **特殊文字フィルタリング** - ファイル名検証の強化
5. **チェックサム検証のデフォルト有効化** - データ整合性保証

### 低優先度

6. **上書き保護オプション** - ユーザビリティ向上
7. **パス深度制限** - リソース保護

---

## セキュリティのベストプラクティス

### 使用時の推奨事項

1. **信頼できないソースからのTARアーカイブは慎重に扱う**
2. **展開前にアーカイブ内容を `list()` で確認**
3. **専用の隔離されたディレクトリに展開**
4. **展開後のファイルパーミッションを確認**

### 実装時の推奨事項

1. **すべてのユーザー入力（ファイル名）を検証**
2. **エラーハンドリングを適切に実装**
3. **リソース制限を設定**
4. **セキュリティアップデートを定期的に確認**

---

## テストカバレッジ

以下のセキュリティテストが実装されています:

### tar.rs モジュール
- `security_test_path_traversal_attack` - パストラバーサル検出
- `security_test_size_mismatch` - サイズ不一致処理
- `security_test_integer_overflow` - 整数オーバーフロー処理
- `security_test_null_byte_injection` - NULLバイトインジェクション
- `security_test_invalid_checksum` - 不正チェックサム処理
- `security_test_symlink_in_archive` - シンボリックリンクフィルタリング
- `security_test_device_file_in_archive` - デバイスファイルフィルタリング
- `security_test_deeply_nested_path` - 深いパス処理
- `security_test_malformed_archive_early_termination` - 破損アーカイブ処理
- `security_test_oversized_name` - 過大な名前フィールド
- `security_test_oversized_prefix` - 過大なプレフィックスフィールド
- `security_test_all_fields_oversized` - すべてのフィールドオーバーサイズ

### lib.rs モジュール
- `security_test_unpack_path_traversal` - unpackのパストラバーサル
- `security_test_unpack_absolute_path` - 絶対パス処理
- `security_test_unpack_large_file_size` - 大きなファイルサイズ
- `security_test_unpack_empty_filename` - 空のファイル名
- `security_test_unpack_special_characters` - 特殊文字処理
- `security_test_pack_symlink_handling` - packのシンボリックリンク処理
- `security_test_unpack_overwrites_existing` - ファイル上書き動作

---

## 報告日

2025年11月28日

## 最終更新日

2025年11月28日

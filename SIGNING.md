# Code Signing and Notarization Setup

This document explains how to configure GitHub Actions for automatic code signing and notarization of macOS binaries.

## Prerequisites

1. **Apple Developer Program Membership** ($99/year)
2. **Developer ID Application Certificate** (obtained from Apple Developer portal)
3. **App-Specific Password** (for notarytool)

## Enable Code Signing

### 1. Enable Signing Variable

Navigate to your repository's **Settings → Secrets and variables → Actions → Variables** and add:

- **Name**: `ENABLE_MACOS_SIGNING`
- **Value**: `true`

This controls whether the signing steps will run during the release build.

## Required GitHub Secrets

Navigate to your repository's **Settings → Secrets and variables → Actions → Secrets** and add the following secrets:

### 1. `APPLE_CERTIFICATE_BASE64`

Your Developer ID Application certificate in base64 format.

**Steps to create:**

1. **キーチェーンアクセスを開く**:
   - アプリケーション → ユーティリティ → キーチェーンアクセス.app
   - または Spotlight で「キーチェーンアクセス」を検索

2. **証明書を見つける**:

   **方法A: キーチェーンアクセスで検索**
   - 左サイドバーで「ログイン」キーチェーンを選択
   - 上部の検索ボックスに「Developer ID Application」と入力
   - 「Developer ID Application: あなたの名前 (TEAM ID)」という項目を探す
   - 証明書の左側の三角形（▶）をクリックして展開すると、秘密鍵が表示される
   - **秘密鍵のアイコンが見えることを確認**（これが重要！）

   **方法B: ターミナルで確認**
   ```bash
   # 利用可能な署名用証明書を一覧表示
   security find-identity -v -p codesigning
   ```

   以下のような出力が表示されます：
   ```
   1) ABCDEF1234567890... "Developer ID Application: Your Name (TEAM12345)"
   ```

3. **証明書をエクスポート**:
   - 証明書（または秘密鍵を含む証明書）を右クリック
   - 「"Developer ID Application: ..."を書き出す...」を選択
   - または、証明書を選択して メニューバー → ファイル → 項目を書き出す...

4. **保存設定**:
   - ファイル名: `Certificates.p12`（任意の名前でOK）
   - 保存場所: デスクトップなど覚えやすい場所
   - ファイルフォーマット: 「個人情報交換 (.p12)」を選択
   - 「保存」をクリック

5. **パスワード設定**:
   - パスワード入力画面が表示される
   - 強力なパスワードを設定（このパスワードは後で`APPLE_CERTIFICATE_PASSWORD`として使用）
   - パスワードを確認のため再入力
   - 「OK」をクリック

6. **キーチェーンのパスワード入力**:
   - Macのログインパスワードを入力して証明書のエクスポートを許可
   - 「常に許可」または「許可」をクリック

7. **base64に変換**:
   ```bash
   # ターミナルで以下を実行（Certificates.p12のパスを適切に変更）
   base64 -i ~/Desktop/Certificates.p12 | pbcopy
   ```
   これでbase64エンコードされた文字列がクリップボードにコピーされます

8. **GitHub Secretに貼り付け**:
   - クリップボードの内容をGitHub Secretの`APPLE_CERTIFICATE_BASE64`に貼り付け

**重要な注意事項**:
- 証明書には必ず秘密鍵が含まれている必要があります（三角形を展開して確認）
- 秘密鍵がない場合は、証明書を再発行するか、元々証明書を作成したMacから取得する必要があります
- .p12ファイルはエクスポート後、安全に保管または削除してください（GitHub Secretに保存後）

### 2. `APPLE_CERTIFICATE_PASSWORD`

The password you used when exporting the .p12 certificate.

### 3. `APPLE_SIGNING_IDENTITY`

Your full signing identity name (without "Developer ID Application:" prefix).

**How to find:**

```bash
security find-identity -v -p codesigning
```

Look for a line like:
```
1) ABCD1234... "Developer ID Application: Your Name (TEAM12345)"
```

Use only the part: `Your Name (TEAM12345)`

### 4. `APPLE_ID`

Your Apple ID email address (used for App Store Connect).

Example: `your-email@example.com`

### 5. `APPLE_TEAM_ID`

Your 10-character Apple Developer Team ID.

**How to find:**
- Visit [Apple Developer Account](https://developer.apple.com/account)
- Look for "Team ID" in the membership details
- Or run: `security find-identity -v -p codesigning` and use the parenthesized value

Example: `TEAM12345`

### 6. `APPLE_APP_SPECIFIC_PASSWORD`

An app-specific password for notarization.

**How to create:**
1. Visit [appleid.apple.com](https://appleid.apple.com)
2. Sign in with your Apple ID
3. Navigate to **Security → App-Specific Passwords**
4. Click **Generate an app-specific password**
5. Label it "GitHub Actions Notarization"
6. Copy the generated password (format: `xxxx-xxxx-xxxx-xxxx`)

## Verification

After setting up all secrets, push a new tag to trigger a release:

```bash
git tag v1.0.4
git push origin v1.0.4
```

Monitor the GitHub Actions workflow. The signing and notarization steps should complete successfully:

- ✅ Import Code-Signing Certificates (macOS)
- ✅ Sign Binary (macOS)
- ✅ Sign bundled dependencies (macOS)
- ✅ Notarize with Apple (macOS)

## Troubleshooting

### Certificate Import Fails

**Error**: "unable to import certificate"

**Solution**: Verify `APPLE_CERTIFICATE_BASE64` is correctly base64-encoded and `APPLE_CERTIFICATE_PASSWORD` is correct.

### Signing Fails

**Error**: "no identity found"

**Solution**: Check `APPLE_SIGNING_IDENTITY` matches exactly (excluding the "Developer ID Application:" prefix).

### Notarization Fails

**Error**: "Invalid credentials" or "Authentication failed"

**Solutions**:
- Verify `APPLE_ID` is correct
- Verify `APPLE_TEAM_ID` is your 10-character team ID
- Regenerate `APPLE_APP_SPECIFIC_PASSWORD` if needed
- Ensure your Apple Developer account is active

### Notarization Timeout

**Error**: "Timeout waiting for notarization"

**Solution**: Apple's notarization service can be slow. The workflow uses `--wait` which should handle this, but if it times out, check the notarization status manually:

```bash
xcrun notarytool history --apple-id "YOUR_APPLE_ID" \
  --team-id "YOUR_TEAM_ID" \
  --password "YOUR_APP_SPECIFIC_PASSWORD"
```

## Security Notes

- **Never commit certificates or passwords to the repository**
- GitHub Secrets are encrypted and only exposed to authorized workflows
- Use repository secrets (not environment secrets) for maximum security
- Rotate app-specific passwords periodically
- Consider using separate certificates for different projects

## Additional Resources

- [Apple Developer Documentation - Notarizing macOS Software](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [GitHub Actions - Encrypted Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [notarytool Documentation](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution/customizing_the_notarization_workflow)

//! Shared HTML helpers for the server-rendered pages (`/block/:index`).

/// Minimal HTML-escaping for any field that could carry user-submitted content
/// (transactions come in via `POST /api/tx`).
pub(crate) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) const BLOCK_PAGE_HEAD: &str = r#"<!doctype html><html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Entropa — block detail</title>
<link rel="icon" href="/assets/favicon.ico">
<style>
:root{color-scheme:dark}
body{margin:0;background:#0a0e14;color:#dbe4f0;font:15px/1.5 -apple-system,system-ui,sans-serif}
.wrap{max-width:760px;margin:0 auto;padding:32px 24px 64px}
.back{color:#7fb8ff;text-decoration:none;font-size:14px}
.back:hover{text-decoration:underline}
h1{margin:16px 0 20px;font-size:28px}
h2{margin:32px 0 12px;font-size:18px;color:#9fb3cc}
table{width:100%;border-collapse:collapse;background:rgba(255,255,255,.03);border:1px solid rgba(255,255,255,.08);border-radius:10px;overflow:hidden}
.fields th,.fields td{padding:10px 14px;text-align:left;border-top:1px solid rgba(255,255,255,.06)}
.fields tr:first-child th,.fields tr:first-child td{border-top:none}
.fields th{color:#8ea0b8;font-weight:500;width:160px}
.txs th,.txs td{padding:8px 14px;text-align:left;border-top:1px solid rgba(255,255,255,.06);font-size:13px}
.txs th{color:#8ea0b8;font-weight:500}
.mono{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px;word-break:break-all;color:#a8d8ff}
.nav{margin-top:28px;display:flex;justify-content:space-between;font-size:14px}
.nav a{color:#7fb8ff;text-decoration:none}
.nav a:hover{text-decoration:underline}
.muted{color:#5a6b80}
.sep{display:none}
</style></head>
"#;

//! 防漏：会显示到界面上的后端文案必须三语（写在 `tr!` 里），不能只有中文。
//!
//! 扫描应用和各库 crate 的源码，找出含中文的字符串字面量，要求它们位于 `tr!(…)` 之内。
//! 例外：注释、`#[cfg(test)]` 修饰的测试代码、只进终端的日志（`eprintln!` / `println!`）、
//! 程序不变量（`expect` / `panic!` 等）；命令行工具 `halokey-cli` 的输出只有中文，不扫；
//! `apps/halokey/src/text.rs` 按调用方给的语言分别写出三语，也不扫。

use std::path::{Path, PathBuf};

/// 允许出现中文字面量的调用：`tr!` 本身，以及不显示到界面上的日志与不变量。
const ALLOWED_CALLS: &[&str] = &[
    "tr!",
    "eprintln!",
    "println!",
    "expect",
    "panic!",
    "unreachable!",
    "assert!",
    "assert_eq!",
    "debug_assert!",
];

const SCANNED: &[&str] = &[
    "apps/halokey/src",
    "crates/hid/src",
    "crates/host/src",
    "crates/input/src",
    "crates/proto/src",
    "crates/runtime/src",
    "crates/wm/src",
];

const SKIPPED_FILES: &[&str] = &["apps/halokey/src/text.rs"];

fn is_cjk(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}')
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// 字符串字面量（起止下标，按字符计）；同时把注释和字面量内部标成"不是代码"。
struct Lexed {
    chars: Vec<char>,
    is_code: Vec<bool>,
    strings: Vec<(usize, usize)>,
}

fn lex(src: &str) -> Lexed {
    let chars: Vec<char> = src.chars().collect();
    let mut is_code = vec![true; chars.len()];
    let mut strings = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                is_code[i] = false;
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                is_code[i] = false;
                i += 1;
            }
            mark(&mut is_code, i, i + 2);
            i += 2;
        } else if c == 'r' && (next == Some('"') || next == Some('#')) && !prev_is_ident(&chars, i)
        {
            // 原始字符串 r"…" / r#"…"#
            let start = i;
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) != Some(&'"') {
                i += 1;
                continue;
            }
            j += 1;
            loop {
                if j >= chars.len() {
                    break;
                }
                if chars[j] == '"' && (0..hashes).all(|h| chars.get(j + 1 + h) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                j += 1;
            }
            mark(&mut is_code, start, j);
            strings.push((start, j));
            i = j;
        } else if c == '"' {
            let start = i;
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '"' {
                if chars[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            j += 1;
            mark(&mut is_code, start, j);
            strings.push((start, j.min(chars.len())));
            i = j;
        } else if c == '\'' {
            // 字符字面量 'x' / '\n'；生命周期 'a 不处理
            if next == Some('\\') && chars.get(i + 3) == Some(&'\'') {
                mark(&mut is_code, i, i + 4);
                i += 4;
            } else if chars.get(i + 2) == Some(&'\'') {
                mark(&mut is_code, i, i + 3);
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    Lexed {
        chars,
        is_code,
        strings,
    }
}

fn prev_is_ident(chars: &[char], i: usize) -> bool {
    i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')
}

fn mark(is_code: &mut [bool], from: usize, to: usize) {
    let to = to.min(is_code.len());
    for v in is_code.iter_mut().take(to).skip(from) {
        *v = false;
    }
}

/// 包住下标 `at` 的最内层未闭合括号前面的调用名，如 `tr!`、`expect`。
fn enclosing_call(lexed: &Lexed, at: usize) -> Option<String> {
    let mut depth = 0i32;
    let mut i = at;
    while i > 0 {
        i -= 1;
        if !lexed.is_code[i] {
            continue;
        }
        match lexed.chars[i] {
            ')' | ']' | '}' => depth += 1,
            '(' | '[' | '{' if depth > 0 => depth -= 1,
            '(' => {
                let mut j = i;
                while j > 0 && lexed.chars[j - 1].is_whitespace() {
                    j -= 1;
                }
                let end = j;
                while j > 0
                    && (lexed.chars[j - 1].is_alphanumeric()
                        || matches!(lexed.chars[j - 1], '_' | '!'))
                {
                    j -= 1;
                }
                return Some(lexed.chars[j..end].iter().collect());
            }
            '[' | '{' => return None,
            _ => {}
        }
    }
    None
}

/// `#[cfg(test)]` 修饰的条目（测试模块、测试函数）所占的范围：从属性到条目末尾的 `}`，
/// 没有花括号的条目（如 `use`）到 `;`。测试模块后面还可能有正式代码，那些照常扫描。
fn test_items(lexed: &Lexed) -> Vec<(usize, usize)> {
    let attr: Vec<char> = "#[cfg(test)]".chars().collect();
    let chars = &lexed.chars;
    let code = |k: usize| lexed.is_code[k];
    let mut ranges = Vec::new();
    let mut i = 0;
    while i + attr.len() <= chars.len() {
        if !(code(i) && chars[i..i + attr.len()] == attr[..]) {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i + attr.len();
        // 找条目体的开头：圆括号、方括号外的第一个 `{` 或 `;`
        let mut nesting = 0i32;
        while j < chars.len() {
            if code(j) {
                match chars[j] {
                    '(' | '[' => nesting += 1,
                    ')' | ']' => nesting -= 1,
                    '{' | ';' if nesting == 0 => break,
                    _ => {}
                }
            }
            j += 1;
        }
        if j < chars.len() && chars[j] == '{' {
            let mut depth = 0i32;
            while j < chars.len() {
                if code(j) {
                    match chars[j] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                j += 1;
            }
        }
        let end = (j + 1).min(chars.len());
        ranges.push((start, end));
        i = end;
    }
    ranges
}

fn offenders(src: &str) -> Vec<String> {
    let lexed = lex(src);
    let tests = test_items(&lexed);
    lexed
        .strings
        .iter()
        .filter(|&&(s, _)| !tests.iter().any(|&(a, b)| (a..b).contains(&s)))
        .filter(|&&(s, e)| lexed.chars[s..e].iter().any(|&c| is_cjk(c)))
        .filter(|&&(s, _)| {
            enclosing_call(&lexed, s).is_none_or(|call| !ALLOWED_CALLS.contains(&call.as_str()))
        })
        .map(|&(s, e)| {
            let line = lexed.chars[..s].iter().filter(|&&c| c == '\n').count() + 1;
            let text: String = lexed.chars[s..e].iter().take(60).collect();
            format!("{line}: {text}")
        })
        .collect()
}

#[test]
fn every_user_facing_backend_message_is_translated() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in SCANNED {
        rust_files(&root.join(dir), &mut files);
    }
    assert!(
        files.len() > 10,
        "扫描目录不对：只找到 {} 个文件",
        files.len()
    );
    let mut problems = Vec::new();
    for file in files {
        let rel = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if SKIPPED_FILES.contains(&rel.as_str()) {
            continue;
        }
        let src = std::fs::read_to_string(&file).unwrap();
        problems.extend(offenders(&src).into_iter().map(|p| format!("{rel}:{p}")));
    }
    assert!(
        problems.is_empty(),
        "这些中文文案会显示到界面上，请改成 tr!(中文, English, 日本語)：\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_scanner_flags_only_chinese_outside_allowed_calls() {
    let src = r#"
fn f(e: &str) -> String {
    // 注释里的"中文"不算
    eprintln!("[halokey] 日志可以是中文");
    let _ = Some(1).expect("不变量可以是中文");
    let _ok = tr!("中文", "English", "日本語");
    let _nested = tr!("中文 {}", "English {}", "日本語 {}", format!("{e}"));
    let _ascii = "plain";
    format!("只有中文：{e}")
}
#[cfg(test)]
mod tests {
    const X: &str = "测试里随便写";
    fn g() { let _ = ["{", "}"]; }
}
#[cfg(test)]
fn helper(_: [u8; 2]) -> &'static str { "测试函数也不算" }
fn after_tests() -> String {
    "测试模块后面的正式代码照样要查".into()
}
"#;
    let found = offenders(src);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("9: \"只有中文"), "{found:?}");
    assert!(found[1].starts_with("19: \"测试模块后面"), "{found:?}");
}

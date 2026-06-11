use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Cross-compilation target for script compilation.
pub enum Target {
    /// 64-bit Windows (`x86_64-pc-windows-msvc`)
    Windows,
    /// 64-bit Linux (`x86_64-unknown-linux-gnu`)
    Linux,
    /// Apple Silicon Mac (`aarch64-apple-darwin`)
    Mac,
    /// Build for the current host platform (no `--target` flag).
    Native,
}

impl Target {
    /// Return the Rust target triple, or `None` for native builds.
    fn triple(&self) -> Option<&'static str> {
        match self {
            Target::Windows => Some("x86_64-pc-windows-msvc"),
            Target::Linux => Some("x86_64-unknown-linux-gnu"),
            Target::Mac => Some("aarch64-apple-darwin"),
            Target::Native => None,
        }
    }

    /// Short human-readable label used in cache keys and build directory names.
    fn label(&self) -> &'static str {
        match self {
            Target::Windows => "windows",
            Target::Linux => "linux",
            Target::Mac => "mac",
            Target::Native => "native",
        }
    }

    /// Return the output binary filename (with `.exe` suffix on Windows).
    fn exe_name(&self, script_name: &str) -> String {
        match self {
            Target::Windows => format!("{}.exe", script_name),
            Target::Native if cfg!(windows) => format!("{}.exe", script_name),
            _ => script_name.to_string(),
        }
    }

    /// Suffix for the temporary build directory.
    fn build_dir_suffix(&self) -> &'static str {
        match self {
            Target::Native => "native",
            t => t.label(),
        }
    }
}

/// Strip Windows long-path prefix (`\\?\`) from a path.
fn clean_path(p: &Path) -> PathBuf {
    let s = p.to_string_lossy().to_string();
    let s = s.trim_start_matches("\\\\?\\");
    PathBuf::from(s)
}

/// Compute a content hash of the script file for cache invalidation.
fn hash_script(script_path: &Path) -> io::Result<u64> {
    let content = fs::read_to_string(script_path)?;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Ok(hasher.finish())
}

/// Path to the hash cache file (a dotfile next to the script).
fn hash_cache_path(script_path: &Path, target_label: &str) -> PathBuf {
    let mut p = script_path.to_path_buf();
    let name = p.file_name().unwrap().to_string_lossy().to_string();
    p.set_file_name(format!(".{}.{}.hash", name, target_label));
    p
}

/// Load a previously cached script hash, if any.
fn load_cached_hash(script_path: &Path, target_label: &str) -> Option<u64> {
    let path = hash_cache_path(script_path, target_label);
    let content = fs::read_to_string(path).ok()?;
    content.trim().parse::<u64>().ok()
}

/// Save a script hash to the cache file.
fn save_cached_hash(script_path: &Path, target_label: &str, hash: u64) {
    let path = clean_path(&hash_cache_path(script_path, target_label));
    let _ = fs::write(path, hash.to_string());
}

/// Compile a `.ns` script into a standalone native binary.
///
/// Steps:
/// 1. Check hash cache — skip if script unchanged and binary exists.
/// 2. Create a temporary Cargo workspace in `target/noo_compile/<name>-<target>/`.
/// 3. Generate `main.rs` that embeds the script (`include_str!`) and runs it.
/// 4. Build with `cargo build --release` (optionally cross-compile).
/// 5. Copy the binary next to the original script.
/// 6. Prompt to run the binary immediately (native only).
pub fn compile(script_path: &Path, target: Target) -> io::Result<()> {
    let script_path = clean_path(&fs::canonicalize(script_path)?);
    let script_name = script_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let out_dir = script_path.parent().unwrap_or(Path::new("."));
    let out_path = out_dir.join(target.exe_name(&script_name));

    let platform = target.label();
    let current_hash = hash_script(&script_path)?;
    let cached = load_cached_hash(&script_path, platform);
    if cached == Some(current_hash) && out_path.exists() {
        let msg = "\nScript unchanged, binary is up to date.\n";
        print!("{msg}");
        return Ok(());
    }

    let noo_root = find_crate_root()?;

    let build_root = noo_root
        .join("target")
        .join("noo_compile")
        .join(format!("{}-{}", script_name, target.build_dir_suffix()));
    let src_dir = build_root.join("src");
    fs::create_dir_all(&src_dir)?;

    let lang_path = out_dir.join("config").join("languages.json");
    let lang_path = if lang_path.exists() {
        clean_path(&fs::canonicalize(&lang_path)?)
    } else {
        let old = out_dir.join("languages.json");
        if old.exists() {
            clean_path(&fs::canonicalize(&old)?)
        } else {
            noo_root.join("config").join("languages.json")
        }
    };

    let noo_path_esc = noo_root.to_string_lossy().replace('\\', "\\\\");
    let cargo_toml = format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         [dependencies]\n\
         nooshell = {{ path = \"{noo}\" }}\n\
         tokio = {{ version = \"1\", features = [\"full\"] }}\n",
        name = script_name,
        noo = noo_path_esc
    );
    fs::write(build_root.join("Cargo.toml"), cargo_toml)?;

    let script_content = fs::read_to_string(&script_path)?;
    let lang_content = fs::read_to_string(&lang_path)?;
    let config_tmp = crate::config::load_from_str(&lang_content).unwrap_or_default();
    let parsed = crate::script::NsScript::from_string(&script_content, &config_tmp)
        .unwrap_or(crate::script::NsScript { lines: Vec::new() });
    let cleanup = crate::script::compute_cleanup_schedule(&parsed.lines);

    let script_path_clean = clean_path(&script_path);
    let lang_path_clean = clean_path(&lang_path);
    let script_path_esc = script_path_clean.to_string_lossy().replace('\\', "\\\\");
    let lang_path_esc = lang_path_clean.to_string_lossy().replace('\\', "\\\\");

    let cleanup_parts: Vec<String> = cleanup
        .iter()
        .map(|(line, cmd)| format!("{line}:{cmd}"))
        .collect();
    let cleanup_str = cleanup_parts.join("|");

    let main_rs = format!(
        "const SCRIPT: &str = include_str!(r\"{s}\");\n\
         const LANGUAGES_JSON: &str = include_str!(r\"{l}\");\n\
         const CLEANUP_STR: &str = \"{c}\";\n\
         \n\
         #[tokio::main]\n\
         async fn main() {{\n\
             let mut cleanup: Vec<(usize, String)> = Vec::new();\n\
             if !CLEANUP_STR.is_empty() {{\n\
                 for entry in CLEANUP_STR.split('|') {{\n\
                     if let Some((line_s, cmd)) = entry.split_once(':') {{\n\
                         if let Ok(line_idx) = line_s.parse::<usize>() {{\n\
                             cleanup.push((line_idx, cmd.to_string()));\n\
                         }}\n\
                     }}\n\
                 }}\n\
             }}\n\
             nooshell::script::NsScript::run_embedded_with_cleanup(\n\
                 SCRIPT,\n\
                 LANGUAGES_JSON,\n\
                 &cleanup,\n\
             )\n\
             .await;\n\
         }}\n",
        s = script_path_esc,
        l = lang_path_esc,
        c = cleanup_str
    );
    fs::write(src_dir.join("main.rs"), main_rs)?;

    let banner = format!("\nCompiling {} for {platform}\n", script_name);
    print!("{banner}");
    let mut cargo_args = vec!["build", "--release"];
    if let Some(triple) = target.triple() {
        let tmsg = format!("  target: {triple}\n");
        print!("{tmsg}");
        cargo_args.push("--target");
        cargo_args.push(triple);
    }

    let status = std::process::Command::new("cargo")
        .args(&cargo_args)
        .current_dir(&build_root)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        eprintln!("\nCompilation failed.\n");
        eprintln!("\nMake sure the target is installed:\n");
        if let Some(triple) = target.triple() {
            eprintln!("  rustup target add {triple}");
        }
        std::process::exit(1);
    }

    let release_dir = match target.triple() {
        Some(t) => build_root.join("target").join(t).join("release"),
        None => build_root.join("target").join("release"),
    };
    let built_bin = release_dir.join(target.exe_name(&script_name));

    fs::copy(&built_bin, &out_path)?;
    let cmsg = format!("\nCompiled: {}\n", clean_path(&out_path).display());
    print!("{cmsg}");

    save_cached_hash(&script_path, platform, current_hash);

    if !matches!(target, Target::Native) {
        return Ok(());
    }

    let rmsg = format!("\nRun {}? [Y/n]: ", clean_path(&out_path).display());
    print!("{rmsg}");
    io::Write::flush(&mut io::stdout())?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim().eq_ignore_ascii_case("n") {
        return Ok(());
    }

    #[cfg(windows)]
    {
        std::process::Command::new("cmd").args(["/c", "cls"]).status().ok();
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("clear").status().ok();
    }

    let err = std::process::Command::new(&out_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    match err {
        Ok(s) if !s.success() => {
            eprintln!("\nBinary exited with code: {:?}\n", s.code());
        }
        Err(e) => {
            eprintln!("\nFailed to run binary: {e}\n");
        }
        _ => {}
    }

    Ok(())
}

/// Walk up from the executable path to find the crate root (`Cargo.toml`).
fn find_crate_root() -> io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let mut dir = exe.parent().unwrap();
    loop {
        if dir.join("Cargo.toml").exists() {
            return Ok(dir.to_path_buf());
        }
        dir = match dir.parent() {
            Some(p) => p,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Cannot find crate root (Cargo.toml)",
                ))
            }
        };
    }
}

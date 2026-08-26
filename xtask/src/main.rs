fn main() -> anyhow::Result<()> {
    let task = std::env::args().nth(1).unwrap_or_else(|| "help".into());
    match task.as_str() {
        "products" => println!("products: linfs (S:/rust/LinFS)"),
        "disk" => println!(
            "disk: LinFS workspace S:/rust/LinFS — run cargo xtask disk --enforce for limits"
        ),
        "loc-audit" => loc_audit()?,
        _ => println!("xtask: {{products|disk|loc-audit}}"),
    }
    Ok(())
}

fn loc_audit() -> anyhow::Result<()> {
    // Count Rust lines vs total (simplified)
    let rs = count_lines("crates", ".rs") + count_lines("xtask", ".rs") + count_lines("src", ".rs");
    let total = rs + count_lines("ui", ".js") + count_lines("ui", ".html");
    let pct = if total == 0 { 100 } else { rs * 100 / total };
    println!("loc-audit: Rust {rs} / total {total} = {pct}%");
    let stretch = std::env::args().any(|a| a == "--stretch-96");
    if stretch && pct < 96 {
        anyhow::bail!("stretch-96 failed: {pct}% < 96%");
    }
    Ok(())
}

fn count_lines(dir: &str, ext: &str) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut n = 0;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            n += count_lines(&p.to_string_lossy(), ext);
        }
        if p.extension().and_then(|s| s.to_str()) == Some(ext.trim_start_matches('.')) {
            if let Ok(s) = std::fs::read_to_string(&p) {
                n += s.lines().count();
            }
        }
    }
    n
}

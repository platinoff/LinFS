fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let stretch = args.iter().any(|a| a=="--stretch-96");
    // delegate to xtask logic
    let rs = count("crates") + count("xtask") + count("src");
    let total = rs + count("ui");
    let pct = if total==0 { 100 } else { rs*100/total };
    println!("linfs-loc-audit: Rust {rs} / total {total} = {pct}%");
    if stretch && pct < 96 { anyhow::bail!("96% stretch failed: {pct}%"); }
    Ok(())
}
fn count(dir: &str) -> usize {
    let Ok(rd)=std::fs::read_dir(dir) else { return 0; };
    let mut n=0;
    for e in rd.flatten() {
        let p=e.path();
        if p.is_dir(){ n+=count(&p.to_string_lossy()); }
        else if p.extension().and_then(|s| s.to_str())==Some("rs") {
            if let Ok(s)=std::fs::read_to_string(&p){ n+=s.lines().count(); }
        } else if p.extension().and_then(|s| s.to_str())==Some("js") || p.extension().and_then(|s| s.to_str())==Some("html") {
            if let Ok(s)=std::fs::read_to_string(&p){ n+=s.lines().count(); }
        }
    }
    n
}

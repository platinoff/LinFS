//! Band 210: criterion bench ls -R 100k + journal commit
//! Run with `cargo bench -p linfs-fs`
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_dir_parse(c: &mut Criterion) {
    c.bench_function("dir_parse_4k", |b| {
        let mut block = vec![0u8; 4096];
        block[0..4].copy_from_slice(&2u32.to_le_bytes());
        block[4..6].copy_from_slice(&12u16.to_le_bytes());
        block[6] = 1;
        block[7] = 2;
        b.iter(|| {
            let _ = linfs_fs::ext4::dir::parse_dir_block(&block).unwrap();
        })
    });
}

criterion_group!(benches, bench_dir_parse);
criterion_main!(benches);

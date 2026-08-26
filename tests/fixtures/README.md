# Fixtures

Generate on Linux:

```bash
bash scripts/mkfixtures.sh
```

Produces: `ext4-plain.img`, `ext4-64bit.img`, `ext4-extent.img`, `ext4-htree-100k.img`, `ext4-needs-recovery.img`, `xfs.img`, `btrfs-single.img`, `f2fs.img`, `luks2-header.bin`
Check in the small ones (<8 MiB); large ones are generated in CI.

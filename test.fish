#!/usr/bin/env fish

set src (mktemp /tmp/src.XXXXXXXXXX)
set mnt_src (mktemp -d /tmp/mnt-src.XXXXXXXXXX)
set tgt (mktemp /tmp/tgt.XXXXXXXXXX)
set mnt_tgt (mktemp -d /tmp/mnt-tgt.XXXXXXXXXX)

echo SRC: $src
echo SRC mount: $mnt_src
echo TGT: $tgt
echo TGT mount: $mnt_tgt

dd if=/dev/zero of=$src bs=1 count=0 seek=512M
dd if=/dev/zero of=$tgt bs=1 count=0 seek=512M
mkfs.btrfs $src
mkfs.btrfs $tgt

doas mount $src $mnt_src/
doas mount $tgt $mnt_tgt/
doas btrfs subvolume create $mnt_src/@
doas mkdir $mnt_src/snapshots
echo aaa | doas tee $mnt_src/@/a
doas btrfs subvolume snapshot -r $mnt_src/@ $mnt_src/snapshots/@
doas btrfs send $mnt_src/snapshots/@ | doas btrfs receive $mnt_tgt
ls -l $mnt_tgt/@
if not test -f $mnt_tgt/@/a
    echo "Initial sync failed - cannot find a on target"
    exit 1
end

echo bbb | doas tee $mnt_src/@/b
doas umount $mnt_src $mnt_tgt

# Incremental

cargo build

doas target/debug/bincbup --source-disk $src --source-mount $mnt_src --target-disk $tgt --target-mount $mnt_tgt --subvolumes @

doas mount $src $mnt_src/
doas mount $tgt $mnt_tgt/
ls -l $mnt_tgt/@
if not test -f $mnt_tgt/@/b
    echo "First backup failed - cannot find b on target"
    exit 1
end
echo ccc | doas tee $mnt_src/@/c
doas umount $mnt_src $mnt_tgt

doas target/debug/bincbup --source-disk $src --source-mount $mnt_src --target-disk $tgt --target-mount $mnt_tgt --subvolumes @

doas mount $tgt $mnt_tgt/
ls -l $mnt_tgt/@
if not test -f $mnt_tgt/@/c
    echo "Second backup failed - cannot find c on target"
    exit 1
end
doas umount $mnt_tgt

rm $src $tgt
rmdir $mnt_src $mnt_tgt

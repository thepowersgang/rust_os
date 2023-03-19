V ?= @

IMGDIR := data/

run_tests: $(IMGDIR)hda.img
	printf "add_disk virt0 $(IMGDIR)hda.img temporary\nmkdir / fat\nmkdir / ext2\nmount /fat virt0p0\nls /fat\nmount /ext2 virt0p1\nls /ext2" | cargo run

$(IMGDIR)hd%_0.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ZERO 1MB $@"
	@# - 1MB of blank space 
	$Vdd if=/dev/zero of=$@ bs=1M count=1 status=noxfer
# First HDD, Partition 1: FAT
$(IMGDIR)hda_1.img: Makefile
	@mkdir -p $(dir $@)
	@echo "[MkDisk] FAT 32MB $@"
	@# - 32MB FAT? partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=32 status=noxfer
	$V/sbin/mkfs.vfat $@
	@# FILES:
	$Vecho "Test content" | mcopy -i $@ - ::/1.txt
# First HDD, Partition 2: ext2
$(IMGDIR)hda_2.img: Makefile
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ext2 16MB $@"
	@# - a 16MB ext2 partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=16 status=noxfer
	$V/sbin/mkfs.ext2 -q -F $@
	@# FILES:
$(IMGDIR)hda.img: Makefile $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img
	@mkdir -p $(dir $@)
	@echo "[MkDisk] mbr $@"
	@# - Commit
	$Vcat $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img > $@
	$Vprintf "$(shell echo $$((1*1024*2)),$$((32*1024*2)),0x83)\n$(shell echo $$((33*1024*2)),+,0x7)" | /sbin/sfdisk --no-reread $@ -u S -f -q > /dev/null

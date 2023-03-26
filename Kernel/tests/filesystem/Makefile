V ?= @

IMGDIR := data/
TESTFILES := $(IMGDIR)testfiles/

run_tests: $(IMGDIR)hda.img
	cargo build
	@echo "add_disk virt0 $(IMGDIR)hda.img temporary" > .testcmds.txt
	@echo "mkdir / fat" >> .testcmds.txt
	@echo "mount /fat virt0p0" >> .testcmds.txt
	@echo "ls /fat" >> .testcmds.txt
	@echo "readback $(TESTFILES)1.txt /fat/1.txt" >> .testcmds.txt
	#@echo "store $(TESTFILES)1.txt /fat/2.txt" >> .testcmds.txt
	#@echo "readback $(TESTFILES)1.txt /fat/2.txt" >> .testcmds.txt
	cargo run < .testcmds.txt
	rm .testcmds.txt
	#printf "add_disk virt0 $(IMGDIR)hda.img temporary\nmkdir / ext2\nmount /ext2 virt0p1\nls /ext2" | cargo run

$(IMGDIR)hd%_0.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ZERO 1MB $@"
	@# - 1MB of blank space 
	$Vdd if=/dev/zero of=$@ bs=1M count=1 status=noxfer
# First HDD, Partition 1: FAT
$(IMGDIR)hda_1.img: Makefile $(TESTFILES)1.txt
	@mkdir -p $(dir $@)
	@echo "[MkDisk] FAT 32MB $@"
	@# - 32MB FAT? partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=32 status=noxfer
	$V/sbin/mkfs.vfat $@
	@# FILES:
	$Vmcopy -i $@ $(TESTFILES)1.txt ::/1.txt
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

# Files for extra testing
$(TESTFILES)1.txt: Makefile
	@mkdir -p $(dir $@)
	echo "Test content" > $@

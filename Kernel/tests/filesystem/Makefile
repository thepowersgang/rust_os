#
#
#
V ?= @
D = $

IMGDIR := data/
TESTFILES := $(IMGDIR)testfiles/
BIN := ../../target/debug/kernel-test-filesystem

.PHONY: build run_tests
run_tests: testlog_fat.log testlog_ext2.log testlog_ntfs.log
run_tests: testlog_ntfs-2.log
build: $(BIN)

testlog_%.log: .testcmds_%.txt $(BIN)
	$(BIN) < $< >$@ 2>&1
#	$(BIN) < $< | tee $@
	
.PHONY: $(BIN)
$(BIN):
	cargo build


.testcmds_%.txt: test_files/%.txt Makefile
	@cat $< | sed 's~%IMGDIR%~$(IMGDIR)~g' | sed 's~%TESTFILES%~$(TESTFILES)~g' > $@

.testcmds_ntfs-2.txt: Makefile data/ntfs2.zdisk
.testcmds_ext2.txt: Makefile $(IMGDIR)hda.img $(TESTFILES)bigfile.dat $(TESTFILES)1.txt
.testcmds_fat.txt: $(TESTFILES)bigfile.dat $(TESTFILES)1.txt

$(IMGDIR)ntfs.img: Makefile
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ZERO 1MB $@"
	@# - 32MB FAT? partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=32 status=noxfer
	$V/sbin/mkfs.ntfs -F -s 512 $@

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
$(IMGDIR)hda_2.img: Makefile $(TESTFILES)1.txt
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ext2 16MB $@"
	@# - a 16MB ext2 partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=16 status=noxfer
#	-q : Quiet
#	-F : Force creation (even if not a block device)
	$V/sbin/mkfs.ext2 -q -F $@
	@# FILES:
	$Vguestfish -a $@ launch : mount /dev/sda / : copy-in $(TESTFILES)1.txt /
$(IMGDIR)hda.img: Makefile $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img
	@mkdir -p $(dir $@)
	@echo "[MkDisk] mbr $@"
	@# - Commit
	$Vcat $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img > $@
	$Vprintf "$(shell echo $$((1*1024*2)),$$((32*1024*2)),0x83)\n$(shell echo $$((33*1024*2)),+,0x7)" | /sbin/sfdisk --no-reread $@ -u S -f -q > /dev/null

# Files for extra testing
$(TESTFILES)1.txt: Makefile
	@mkdir -p $(dir $@)
	$Vecho "Test content" > $@
$(TESTFILES)bigfile.dat: Makefile
	@mkdir -p $(dir $@)
	$Vdd if=/dev/zero of=$@ bs=512 count=7

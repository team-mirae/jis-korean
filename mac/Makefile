# Builds JISKorean.app (universal arm64 + x86_64) without Xcode: Command Line Tools are enough.
#   make            build build/JISKorean.app
#   make install    copy it to ~/Library/Input Methods, register it, and restart it

ENGINE     := ../engine
HEADER     := $(ENGINE)/include/jis_korean.h
BUILD      := build
APP        := $(BUILD)/JISKorean.app
INSTALL_TO := $(HOME)/Library/Input Methods
MIN_MACOS  := 13.0
SOURCES    := $(wildcard Sources/*.swift)
CARGO      := $(shell command -v cargo || echo $(HOME)/.cargo/bin/cargo)

export MACOSX_DEPLOYMENT_TARGET := $(MIN_MACOS)

.PHONY: all engine install uninstall clean

all: $(APP)

# --- engine: Rust static library, one per arch, merged with lipo

$(BUILD)/libjis_korean.a: $(wildcard $(ENGINE)/src/*.rs) $(ENGINE)/Cargo.toml
	@mkdir -p $(BUILD)
	$(CARGO) build --manifest-path $(ENGINE)/Cargo.toml --release --target aarch64-apple-darwin
	$(CARGO) build --manifest-path $(ENGINE)/Cargo.toml --release --target x86_64-apple-darwin
	lipo -create -output $@ \
		$(ENGINE)/target/aarch64-apple-darwin/release/libjis_korean.a \
		$(ENGINE)/target/x86_64-apple-darwin/release/libjis_korean.a

engine: $(BUILD)/libjis_korean.a

# --- Swift: one binary per arch, merged with lipo

$(BUILD)/%/JISKorean: $(SOURCES) $(HEADER) $(BUILD)/libjis_korean.a
	@mkdir -p $(dir $@)
	swiftc -O -swift-version 5 -target $*-apple-macos$(MIN_MACOS) \
		-import-objc-header $(HEADER) \
		-framework Cocoa -framework InputMethodKit -framework Carbon \
		$(SOURCES) $(BUILD)/libjis_korean.a -o $@

$(BUILD)/JISKorean: $(BUILD)/arm64/JISKorean $(BUILD)/x86_64/JISKorean
	lipo -create -output $@ $^

# --- icon

$(BUILD)/icon.tiff: tools/make-icon.swift
	@mkdir -p $(BUILD)/icon
	swift tools/make-icon.swift $(BUILD)/icon
	tiffutil -cathidpicheck $(BUILD)/icon/icon.png $(BUILD)/icon/icon@2x.png -out $@

# --- bundle

DICT := $(ENGINE)/dict

$(APP): $(BUILD)/JISKorean $(BUILD)/icon.tiff Info.plist $(DICT)/hanja.tsv
	rm -rf $@
	mkdir -p $@/Contents/MacOS $@/Contents/Resources
	cp Info.plist $@/Contents/Info.plist
	cp $(BUILD)/JISKorean $@/Contents/MacOS/JISKorean
	cp $(BUILD)/icon.tiff $@/Contents/Resources/icon.tiff
	cp $(DICT)/hanja.tsv $(DICT)/LICENSE-libhangul-hanja.txt $(DICT)/LICENSE-unicode.txt $@/Contents/Resources/
	codesign --force --sign - $@

install: $(APP)
	@killall JISKorean 2>/dev/null || true
	mkdir -p "$(INSTALL_TO)"
	rm -rf "$(INSTALL_TO)/JISKorean.app"
	cp -R $(APP) "$(INSTALL_TO)/"
	swift tools/register.swift "$(INSTALL_TO)/JISKorean.app"

uninstall:
	@killall JISKorean 2>/dev/null || true
	rm -rf "$(INSTALL_TO)/JISKorean.app"

clean:
	rm -rf $(BUILD)

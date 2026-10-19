# Builds JISKorean.app (universal arm64 + x86_64) without Xcode: Command Line Tools are enough.
#   make                 build build/full/JISKorean.app (with the predictive layer)
#   make PREDICT=0       build build/lite/JISKorean.app: the predictive layer stripped out entirely
#   make install         copy it to ~/Library/Input Methods, register it, and restart it
#   make dist            build/<variant>/JISKorean-<version>[-lite].zip for a release

# 1: with prediction (engine `predict` feature, spec §10). 0: the lean deterministic engine only.
PREDICT    ?= 1
ifeq ($(PREDICT),1)
  VARIANT        := full
  CARGO_FEATURES :=
  SWIFT_DEFINES  := -D PREDICT
  DIST_SUFFIX    :=
else
  VARIANT        := lite
  CARGO_FEATURES := --no-default-features
  SWIFT_DEFINES  :=
  DIST_SUFFIX    := -lite
endif

ENGINE     := ../engine
HEADER     := $(ENGINE)/include/jis_korean.h
BUILD      := build/$(VARIANT)
APP        := $(BUILD)/JISKorean.app
TARGET_DIR := $(ENGINE)/target/mac-$(VARIANT)
INSTALL_TO := $(HOME)/Library/Input Methods
MIN_MACOS  := 13.0
SOURCES    := $(wildcard Sources/*.swift)
CARGO      := $(shell command -v cargo || echo $(HOME)/.cargo/bin/cargo)

export MACOSX_DEPLOYMENT_TARGET := $(MIN_MACOS)

.PHONY: all engine install uninstall dist clean

all: $(APP)

# --- engine: Rust static library, one per arch, merged with lipo

$(BUILD)/libjis_korean.a: $(wildcard $(ENGINE)/src/*.rs) $(ENGINE)/Cargo.toml
	@mkdir -p $(BUILD)
	$(CARGO) build --manifest-path $(ENGINE)/Cargo.toml --release $(CARGO_FEATURES) --target-dir $(TARGET_DIR) --target aarch64-apple-darwin
	$(CARGO) build --manifest-path $(ENGINE)/Cargo.toml --release $(CARGO_FEATURES) --target-dir $(TARGET_DIR) --target x86_64-apple-darwin
	lipo -create -output $@ \
		$(TARGET_DIR)/aarch64-apple-darwin/release/libjis_korean.a \
		$(TARGET_DIR)/x86_64-apple-darwin/release/libjis_korean.a

engine: $(BUILD)/libjis_korean.a

# --- Swift: one binary per arch, merged with lipo

$(BUILD)/%/JISKorean: $(SOURCES) $(HEADER) $(BUILD)/libjis_korean.a
	@mkdir -p $(dir $@)
	swiftc -O -swift-version 5 $(SWIFT_DEFINES) -target $*-apple-macos$(MIN_MACOS) \
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

DICT    := $(ENGINE)/dict
PREDICT_FILES := $(ENGINE)/predict/ranker.bin $(ENGINE)/predict/common-en.txt $(ENGINE)/predict/common-ko.txt \
                 $(ENGINE)/predict/english-nouns.txt $(ENGINE)/predict/LICENSE-wordnet.txt \
                 $(ENGINE)/predict/LICENSE-corpora.txt

$(APP): $(BUILD)/JISKorean $(BUILD)/icon.tiff Info.plist $(DICT)/hanja.tsv
	rm -rf $@
	mkdir -p $@/Contents/MacOS $@/Contents/Resources
	cp Info.plist $@/Contents/Info.plist
	cp $(BUILD)/JISKorean $@/Contents/MacOS/JISKorean
	cp $(BUILD)/icon.tiff $@/Contents/Resources/icon.tiff
	cp $(DICT)/hanja.tsv $(DICT)/LICENSE-libhangul-hanja.txt $(DICT)/LICENSE-unicode.txt $@/Contents/Resources/
ifeq ($(PREDICT),1)
	mkdir -p $@/Contents/Resources/predict
	cp $(PREDICT_FILES) $@/Contents/Resources/predict/
endif
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

VERSION := $(shell plutil -extract CFBundleShortVersionString raw Info.plist)

ZIP := $(BUILD)/JISKorean-$(VERSION)$(DIST_SUFFIX).zip

dist: $(APP)
	rm -f $(ZIP)
	ditto -c -k --keepParent $(APP) $(ZIP)
	@echo $(ZIP)

clean:
	rm -rf build

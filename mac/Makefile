# Builds JISKorean.app (universal arm64 + x86_64) without Xcode: Command Line Tools are enough.
#   make                 build build/full/JISKorean.app (with the predictive layer)
#   make PREDICT=0       build build/lite/JISKorean.app: the predictive layer stripped out entirely
#   make LEGACY_PREDICT=1  build build/full-legacy/JISKorean.app: predictions ranked by the rules
#                        alone (spec §10.3), without the learned ranker (§10.13) or its model
#   make ENGLISH=1       also the English input modes (JIS English, US and UK English, JIS Dvorak and
#                        JIS Colemak; spec §3.6.2, §3.6.4):
#                        build/full-english/JISKorean.app (combines with PREDICT=0: lite-english)
#   make install         copy it to ~/Library/Input Methods, register it, and restart it
#                        (with ENGLISH=1, also a login agent that starts it at login)
#   make dist            build/<variant>/JISKorean-<version>[-lite].zip for a release

# 1: with prediction (engine `predict` feature, spec §10). 0: the lean deterministic engine only.
PREDICT    ?= 1
# With PREDICT=1: 1 ranks predictions by the rules alone (engine `legacy-predict` feature, spec
# §10.13); 0 (the default) adds the learned ranker and its model, predict/learned.bin.
LEGACY_PREDICT ?= 0
ifeq ($(PREDICT)$(LEGACY_PREDICT),11)
  VARIANT        := full-legacy
  CARGO_FEATURES := --features legacy-predict
  SWIFT_DEFINES  := -D PREDICT
  DIST_SUFFIX    := -legacy
else ifeq ($(PREDICT),1)
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

# 1: add JIS English (engine `english` feature, spec §3.6.2). 0: the Korean modes only.
ENGLISH    ?= 0
ifeq ($(ENGLISH),1)
  VARIANT        := $(VARIANT)-english
  CARGO_FEATURES += --features english
  SWIFT_DEFINES  += -D ENGLISH
  DIST_SUFFIX    := $(DIST_SUFFIX)-english
endif

ENGINE     := ../engine
HEADER     := $(ENGINE)/include/jis_korean.h
BUILD      := build/$(VARIANT)
APP        := $(BUILD)/JISKorean.app
TARGET_DIR := $(ENGINE)/target/mac-$(VARIANT)
INSTALL_TO := $(HOME)/Library/Input Methods
LOGIN_AGENT := $(HOME)/Library/LaunchAgents/me.sjun.inputmethod.JISKorean.login.plist
MIN_MACOS  := 13.0
SOURCES    := $(wildcard Sources/*.swift)
CARGO      := $(shell command -v cargo || echo $(HOME)/.cargo/bin/cargo)

# Code signing: your Apple Development certificate if the keychain has one (sign in to Xcode → Settings
# → Accounts, then Manage Certificates), else ad hoc. Permissions macOS grants (Input Monitoring and
# Accessibility, for the system-wide カタカナ / F18 keys) follow the signature: with a certificate
# they survive rebuilds; ad hoc, every build is a new app to macOS and has to be granted again.
# Override with `make SIGN_IDENTITY="..."`.
SIGN_IDENTITY ?= $(or $(shell security find-identity -v -p codesigning 2>/dev/null | sed -n 's/.*"\(Apple Development: [^"]*\)".*/\1/p' | head -1),-)

LSREGISTER := /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister

export MACOSX_DEPLOYMENT_TARGET := $(MIN_MACOS)

.PHONY: all engine install uninstall dist clean

all: $(APP)

# Spotlight would register build copies with LaunchServices (see install); keep it out of build/.
$(shell mkdir -p build && touch build/.metadata_never_index)

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
		-framework Cocoa -framework InputMethodKit -framework Carbon -framework IOKit \
		$(SOURCES) $(BUILD)/libjis_korean.a -o $@

$(BUILD)/JISKorean: $(BUILD)/arm64/JISKorean $(BUILD)/x86_64/JISKorean
	lipo -create -output $@ $^

# --- icons: the logo and one badge per mode, drawn in docs/logo (make_logo.py, render.sh) and
#     committed as 16 px and 32 px PNGs; each pair becomes one .tiff

ICONS := $(BUILD)/taegeuk.tiff $(BUILD)/mode-a.tiff $(BUILD)/mode-b.tiff $(BUILD)/mode-iso.tiff
ifeq ($(ENGLISH),1)
  ICONS += $(BUILD)/mode-a-qwerty.tiff $(BUILD)/mode-b-qwerty.tiff $(BUILD)/mode-cj-qwerty.tiff \
           $(BUILD)/mode-a-dvorak.tiff $(BUILD)/mode-a-colemak.tiff
endif

$(BUILD)/%.tiff: Resources/icons/%.png Resources/icons/%@2x.png
	@mkdir -p $(BUILD)
	tiffutil -cathidpicheck Resources/icons/$*.png Resources/icons/$*@2x.png -out $@

# --- Keyboard Viewer layouts, one per mode: its legends on ABC (JIS for Mode A, ANSI for Mode B,
#     ISO for Mode C),
#     from the engine's layout tables. Mode n's legends are legends-n.tsv.

$(BUILD)/legends-%.tsv: $(wildcard $(ENGINE)/src/*.rs) $(ENGINE)/examples/legends.rs
	@mkdir -p $(BUILD)
	$(CARGO) run -q --manifest-path $(ENGINE)/Cargo.toml $(CARGO_FEATURES) --example legends --target-dir $(TARGET_DIR) -- $* > $@

$(BUILD)/make-keylayout: tools/make-keylayout/main.swift Sources/KeyMap.swift $(HEADER)
	@mkdir -p $(BUILD)
	swiftc -O -import-objc-header $(HEADER) Sources/KeyMap.swift tools/make-keylayout/main.swift -o $@

$(BUILD)/JISKorean.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-0.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-0.tsv $@ JISKorean JIS -23456

$(BUILD)/JISKoreanANSI.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-1.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-1.tsv $@ JISKoreanANSI ANSI -23457

$(BUILD)/JISKoreanISO.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-2.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-2.tsv $@ JISKoreanISO ISO -23458

# The English modes: their symbols on any keyboard, and capitals under Caps Lock (turned on with a
# modifier: plain Caps Lock is JIS Korean's switch, spec §8.2.1). JIS English (3), US
# English (9), UK English (10), JIS Dvorak (7) and JIS Colemak (8). make-keylayout takes ⌥, ⌘ and
# Ctrl from ABC, so shortcuts stay on QWERTY in Dvorak and Colemak too (spec §3.6.4).
$(BUILD)/JISEnglish.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-3.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-3.tsv $@ JISEnglish JIS -23459 caps

$(BUILD)/USEnglish.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-9.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-9.tsv $@ USEnglish ANSI -23460 caps

$(BUILD)/UKEnglish.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-10.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-10.tsv $@ UKEnglish ISO -23461 caps

$(BUILD)/JISDvorak.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-7.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-7.tsv $@ JISDvorak JIS -23462 caps

$(BUILD)/JISColemak.keylayout: $(BUILD)/make-keylayout $(BUILD)/legends-8.tsv
	$(BUILD)/make-keylayout $(BUILD)/legends-8.tsv $@ JISColemak JIS -23463 caps

KEYLAYOUTS := $(BUILD)/JISKorean.keylayout $(BUILD)/JISKoreanANSI.keylayout $(BUILD)/JISKoreanISO.keylayout
ifeq ($(ENGLISH),1)
  KEYLAYOUTS += $(BUILD)/JISEnglish.keylayout $(BUILD)/USEnglish.keylayout $(BUILD)/UKEnglish.keylayout \
                $(BUILD)/JISDvorak.keylayout $(BUILD)/JISColemak.keylayout
endif

# An earlier build installed the layout as its own bundle here; install removes it.
LAYOUTS_TO    := $(HOME)/Library/Keyboard Layouts

# --- bundle

DICT    := $(ENGINE)/dict
PREDICT_FILES := $(ENGINE)/predict/ranker.bin $(ENGINE)/predict/common-en.txt $(ENGINE)/predict/common-ko.txt \
                 $(ENGINE)/predict/english-nouns.txt $(ENGINE)/predict/LICENSE-wordnet.txt \
                 $(ENGINE)/predict/LICENSE-corpora.txt
ifneq ($(LEGACY_PREDICT),1)
  PREDICT_FILES += $(ENGINE)/predict/learned.bin
endif

$(APP): $(BUILD)/JISKorean $(ICONS) Info.plist $(DICT)/hanja.tsv $(KEYLAYOUTS) \
       Resources/JISKoreanKeyboardLayouts-Info.plist Resources/en.lproj/InfoPlist.strings Resources/ko.lproj/InfoPlist.strings
	rm -rf $@
	mkdir -p $@/Contents/MacOS $@/Contents/Resources
	cp Info.plist $@/Contents/Info.plist
	cp $(BUILD)/JISKorean $@/Contents/MacOS/JISKorean
	cp $(ICONS) $@/Contents/Resources/
	cp $(DICT)/hanja.tsv $(DICT)/LICENSE-libhangul-hanja.txt $(DICT)/LICENSE-unicode.txt $@/Contents/Resources/
	cp $(KEYLAYOUTS) Resources/JISKoreanKeyboardLayouts-Info.plist $@/Contents/Resources/
ifeq ($(ENGLISH),1)
	@# JIS English (spec §3.6.2, §8.2.1): an English input mode of this input method, Roman script,
	@# so System Settings lists it under English and 英数 can select it.
	sh tools/add-english-mode.sh $@/Contents
endif
	cp -R Resources/en.lproj Resources/ko.lproj $@/Contents/Resources/
ifeq ($(PREDICT),1)
	mkdir -p $@/Contents/Resources/predict
	cp $(PREDICT_FILES) $@/Contents/Resources/predict/
endif
	codesign --force --sign "$(SIGN_IDENTITY)" $@

install: $(APP)
	@killall JISKorean 2>/dev/null || true
	mkdir -p "$(INSTALL_TO)"
	@# Swap the bundle in with renames: while the path is missing (an uninstall followed by a long
	@# rebuild), macOS drops the input source from running apps and re-registering doesn't bring it
	@# back (docs/troubleshooting/macos-input-method-not-launching.md).
	rm -rf "$(INSTALL_TO)/.JISKorean.app.new" "$(INSTALL_TO)/.JISKorean.app.old"
	ditto $(APP) "$(INSTALL_TO)/.JISKorean.app.new"
	if [ -d "$(INSTALL_TO)/JISKorean.app" ]; then mv "$(INSTALL_TO)/JISKorean.app" "$(INSTALL_TO)/.JISKorean.app.old"; fi
	mv "$(INSTALL_TO)/.JISKorean.app.new" "$(INSTALL_TO)/JISKorean.app"
	rm -rf "$(INSTALL_TO)/.JISKorean.app.old"
	rm -rf "$(LAYOUTS_TO)/JISKorean.bundle"
	@# Only the installed copy may be known to LaunchServices: input methods are launched by bundle
	@# id, and a build copy (or a deleted one) can be picked instead, so the input method never starts.
	@for app in build/JISKorean.app build/full/JISKorean.app build/lite/JISKorean.app \
		build/full-english/JISKorean.app build/lite-english/JISKorean.app build/full-legacy/JISKorean.app \
		build/full-legacy-english/JISKorean.app; do \
		$(LSREGISTER) -u "$(CURDIR)/$$app" 2>/dev/null || true; done
	$(LSREGISTER) -f "$(INSTALL_TO)/JISKorean.app"
	swift tools/register.swift "$(INSTALL_TO)/JISKorean.app"
ifeq ($(ENGLISH),1)
	@# Start it at login: the カタカナ / F18 tap lives in its process, which macOS otherwise starts only
	@# once JIS Korean is selected somewhere (tools/login-agent.plist).
	mkdir -p "$(dir $(LOGIN_AGENT))"
	cp tools/login-agent.plist "$(LOGIN_AGENT)"
else
	rm -f "$(LOGIN_AGENT)"
endif

# To reinstall, run install alone: uninstall first leaves the input source broken in running apps
# until it is removed and re-added in System Settings.
uninstall:
	@killall JISKorean 2>/dev/null || true
	rm -rf "$(INSTALL_TO)/JISKorean.app" "$(LAYOUTS_TO)/JISKorean.bundle" "$(LOGIN_AGENT)"

VERSION := $(shell plutil -extract CFBundleShortVersionString raw Info.plist)

ZIP := $(BUILD)/JISKorean-$(VERSION)$(DIST_SUFFIX).zip

dist: $(APP)
	rm -f $(ZIP)
	ditto -c -k --keepParent $(APP) $(ZIP)
	@echo $(ZIP)

clean:
	rm -rf build

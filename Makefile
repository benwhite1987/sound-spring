PREFIX ?= /usr/local
DESTDIR ?=
QMAKE ?= qmake6
ICON_NAME := io.github.benwhite1987.SoundSpring
ICON_SIZES := 16 22 24 32 48 64 128 256 512
MAGICK ?= magick
VERSION ?= $(shell sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' Cargo.toml | head -1)

.PHONY: all build icons install clean package package-ubuntu test-packages test-packages-quick

all: build

build:
	QMAKE=$(QMAKE) cargo build --release

# Regenerate Freedesktop hicolor icons from resources/icons/source.png (requires ImageMagick).
icons:
	@set -e; \
	for size in $(ICON_SIZES); do \
		mkdir -p resources/icons/hicolor/$${size}x$${size}/apps; \
		$(MAGICK) resources/icons/source.png -resize $${size}x$${size} \
			resources/icons/hicolor/$${size}x$${size}/apps/$(ICON_NAME).png; \
	done

install: build
	install -D target/release/sound-spring $(DESTDIR)$(PREFIX)/bin/sound-spring
	install -D resources/sound-spring.desktop $(DESTDIR)$(PREFIX)/share/applications/sound-spring.desktop
	@set -e; \
	for size in $(ICON_SIZES); do \
		install -D resources/icons/hicolor/$${size}x$${size}/apps/$(ICON_NAME).png \
			$(DESTDIR)$(PREFIX)/share/icons/hicolor/$${size}x$${size}/apps/$(ICON_NAME).png; \
	done

# Build .deb, .rpm, and AppImage into dist/ (requires nfpm + linuxdeploy tooling).
# Use plain `make` (not $(MAKE)): some IDE environments set MAKE to a non-make binary.
# Host-built artifacts may require a newer glibc than Ubuntu/Fedora guests; use
# `make package-ubuntu` (Docker) before `make test-packages` on rolling hosts.
package: build
	@command -v nfpm >/dev/null || { echo "nfpm not found; see packaging/README.md"; exit 1; }
	rm -rf staging
	DESTDIR=staging PREFIX=/usr make install
	mkdir -p dist
	VERSION=$(VERSION) nfpm package -f packaging/nfpm/nfpm.yaml -p deb --target dist
	VERSION=$(VERSION) nfpm package -f packaging/nfpm/nfpm.yaml -p rpm --target dist
	VERSION=$(VERSION) QMAKE=$(QMAKE) packaging/appimage/build-appimage.sh

# Build packages inside Ubuntu 24.04 (matches GitHub release glibc).
package-ubuntu:
	packaging/vm-tests/package-ubuntu.sh

# Build packages then smoke-test installs in QEMU cloud VMs.
# On CachyOS/Arch, prefer: make package-ubuntu && make test-packages-quick
test-packages: package
	packaging/vm-tests/run.sh

# Reuse existing dist/ artifacts for VM smoke tests.
test-packages-quick:
	packaging/vm-tests/run.sh

clean:
	cargo clean
	rm -rf staging AppDir dist

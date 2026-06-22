PREFIX ?= /usr/local
DESTDIR ?=
QMAKE ?= qmake6
ICON_NAME := io.github.benwhite1987.SoundSpring
ICON_SIZES := 16 22 24 32 48 64 128 256 512
MAGICK ?= magick

.PHONY: all build icons install clean

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

clean:
	cargo clean

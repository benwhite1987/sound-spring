PREFIX ?= /usr/local
DESTDIR ?=
QMAKE ?= qmake6

.PHONY: all build install clean

all: build

build:
	QMAKE=$(QMAKE) cargo build --release

install: build
	install -D target/release/sound-spring $(DESTDIR)$(PREFIX)/bin/sound-spring
	install -D resources/sound-spring.desktop $(DESTDIR)$(PREFIX)/share/applications/sound-spring.desktop

clean:
	cargo clean

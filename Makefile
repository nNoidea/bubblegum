.PHONY: all binary appimage dev shell

all: binary

binary:
	./run.sh --build

appimage:
	./run.sh --appimage

dev:
	./run.sh --dev

shell:
	./run.sh --dev

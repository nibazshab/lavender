MUSL_LINK="https://musl.cc/x86_64-linux-musl-cross.tgz"
MUSL_DIR=musllib

CC="$(PWD)/$(MUSL_DIR)/bin/x86_64-linux-musl-gcc"

FLAG="-s -w --extldflags '-static' \
 -X github.com/nibazshab/webnote/cmd/flag.Version=$(VERSION)"

CGO_ENABLED=1
GOOS=linux
GOARCH=amd64

VERSION=$(shell git describe --abbrev=0 --tags)

all: build

golib:
	go mod tidy

cclib:
	if [ ! -d "$(MUSL_DIR)" ]; then \
		mkdir -p $(MUSL_DIR) && \
		wget -O $(MUSL_DIR).tgz $(MUSL_LINK) && \
		tar -zxvf $(MUSL_DIR).tgz --strip-components=1 -C $(MUSL_DIR) && \
		rm $(MUSL_DIR).tgz; \
	fi

build: golib cclib
	CGO_ENABLED=$(CGO_ENABLED) GOOS=$(GOOS) GOARCH=$(GOARCH) CC=$(CC) go build -ldflags=$(FLAG)

clean:
	rm -f webnote

cleanall:
	rm -rf $(MUSL_DIR) webnote

.PHONY: all golib cclib build clean cleanall

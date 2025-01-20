package flag

import (
	"flag"
	"fmt"
	"os"
)

var (
	Version string
	Port    *string
	Path    *string
)

const (
	port = "10003"
	path = "webnote_data"
)

func Init() {
	Port = flag.String("port", port, "server port")
	Path = flag.String("path", path, "data directory")
	_v := flag.Bool("v", false, "version")

	flag.Parse()

	if *_v {
		fmt.Printf("Version %s\nVisit github.com/nibazshab/webnote", Version)
		os.Exit(0)
	}
}

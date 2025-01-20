package path

import (
	"log"
	"os"
	"path/filepath"

	"github.com/nibazshab/webnote/cmd/flag"
)

var dataPath string

func GetFilePath(filename string) string {
	if dataPath == "" {
		if filepath.IsAbs(*flag.Path) {
			dataPath = filepath.Clean(*flag.Path)
		} else {
			exePath, err := os.Executable()
			if err != nil {
				log.Fatalf("data path error: %v", err)
			}
			dataPath = filepath.Join(filepath.Dir(exePath), *flag.Path)
		}

		if _, err := os.Stat(dataPath); os.IsNotExist(err) {
			err = os.MkdirAll(dataPath, 0o755)
			if err != nil {
				log.Fatalf("data path create error: %v", err)
			}
		}
	}

	filePath := filepath.Join(dataPath, filename)
	return filePath
}

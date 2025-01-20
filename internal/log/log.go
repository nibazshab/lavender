package log

import (
	"io"
	"log"
	"os"

	"github.com/gin-gonic/gin"

	"github.com/nibazshab/webnote/internal/path"
	"github.com/nibazshab/webnote/pkg/util"
)

const logFileName = "log.log"

var logFile string

func Init() {
	logFile = path.GetFilePath(logFileName)

	file, err := os.OpenFile(logFile, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		log.Fatalf("log open error: %v", err)
	}

	defer func(file *os.File) {
		err = file.Close()
		if err != nil {
			log.Fatalf("log close error: %v", err)
		}
	}(file)
}

func Logging(c *gin.Context, id string, msg rune) {
	file, err := os.OpenFile(logFile, os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		log.Printf("log open error: %v", err)
	}

	defer func(file *os.File) {
		err = file.Close()
		if err != nil {
			log.Printf("log close error: %v", err)
		}
	}(file)

	multiWriter := io.MultiWriter(os.Stdout, file)
	log.SetOutput(multiWriter)
	log.Printf("%s | %c | %s | %s", id, msg, util.GetUserIP(c.Request), util.GetUserUA(c.Request))
}

func Fatalf(format string, v ...any) {
	log.Fatalf(format, v...)
}

func Printf(format string, v ...any) {
	log.Printf(format, v...)
}

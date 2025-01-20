package cmd

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"

	"github.com/gin-gonic/gin"

	"github.com/nibazshab/webnote/cmd/flag"
	"github.com/nibazshab/webnote/internal/db"
	"github.com/nibazshab/webnote/internal/log"
	"github.com/nibazshab/webnote/router"
)

func Start() {
	flag.Init()
	db.Init()
	log.Init()

	// args := os.Args

	httpServer()
}

func httpServer() {
	ch := make(chan os.Signal, 1)
	signal.Notify(ch, syscall.SIGINT, syscall.SIGTERM)

	defer func() {
		err := db.Close()
		if err != nil {
			log.Fatalf("db close error: %v", err)
		}
	}()

	gin.SetMode(gin.ReleaseMode)
	r := gin.New()

	router.Router(r)

	fmt.Printf("webnote %s\n", flag.Version)
	log.Printf("start HTTP server @ 0.0.0.0:%s\n", *flag.Port)
	go func() {
		if err := r.Run(":" + *flag.Port); err != nil {
			log.Fatalf("start error: %v", err)
		}
	}()

	<-ch
}
